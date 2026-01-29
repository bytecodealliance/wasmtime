use super::address_transform::AddressTransform;
use super::attr::{EntryAttributesContext, clone_die_attributes};
use super::debug_transform_logging::{
    dbi_log, log_begin_input_die, log_end_output_die, log_end_output_die_skipped,
    log_get_cu_summary,
};
use super::expression::compile_expression;
use super::line_program::clone_line_program;
use super::range_info_builder::RangeInfoBuilder;
use super::synthetic::ModuleSyntheticUnit;
use super::utils::{append_vmctx_info, resolve_die_ref};
use crate::debug::{Compilation, Reader};
use cranelift_codegen::ir::Endianness;
use cranelift_codegen::isa::TargetIsa;
use gimli::AttributeValue;
use gimli::write;
use std::collections::HashSet;
use wasmtime_environ::StaticModuleIndex;
use wasmtime_environ::error::{Context, Error};
use wasmtime_versioned_export_macros::versioned_stringify_ident;

#[derive(Debug)]
pub struct InheritedAttr<T> {
    stack: Vec<(isize, T)>,
}

impl<T> InheritedAttr<T> {
    fn new() -> Self {
        InheritedAttr { stack: Vec::new() }
    }

    fn update(&mut self, depth: isize) {
        while !self.stack.is_empty() && self.stack.last().unwrap().0 >= depth {
            self.stack.pop();
        }
    }

    pub fn push(&mut self, depth: isize, value: T) {
        self.stack.push((depth, value));
    }

    pub fn top(&self) -> Option<&T> {
        self.stack.last().map(|entry| &entry.1)
    }

    pub fn top_with_depth_mut(&mut self, depth: isize) -> Option<&mut T> {
        self.stack
            .last_mut()
            .filter(|entry| entry.0 == depth)
            .map(|entry| &mut entry.1)
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

fn get_base_type_name(type_entry: &write::ConvertUnitEntry<Reader<'_>>) -> Result<String, Error> {
    // FIXME remove recursion.
    if let Some(die_ref) = type_entry.attr_value(gimli::DW_AT_type) {
        if let Some(ref die) = resolve_die_ref(type_entry.read_unit, &die_ref)? {
            if let Some(value) = die.attr_value(gimli::DW_AT_name) {
                return Ok(String::from(die.read_unit.attr_string(value)?.to_string()?));
            }
            match die.tag {
                gimli::DW_TAG_const_type => {
                    return Ok(format!("const {}", get_base_type_name(die)?));
                }
                gimli::DW_TAG_pointer_type => {
                    return Ok(format!("{}*", get_base_type_name(die)?));
                }
                gimli::DW_TAG_reference_type => {
                    return Ok(format!("{}&", get_base_type_name(die)?));
                }
                gimli::DW_TAG_array_type => {
                    return Ok(format!("{}[]", get_base_type_name(die)?));
                }
                _ => (),
            }
        }
    }
    Ok(String::from("??"))
}

enum WebAssemblyPtrKind {
    Reference,
    Pointer,
}

/// Replaces WebAssembly pointer type DIE with the wrapper
/// which natively represented by offset in a Wasm memory.
///
/// `pointer_type_entry` is a DW_TAG_pointer_type entry (e.g. `T*`),
/// which refers its base type (e.g. `T`), or is a
/// DW_TAG_reference_type (e.g. `T&`).
///
/// The generated wrapper is a structure that contains only the
/// `__ptr` field. The utility operators overloads is added to
/// provide better debugging experience.
///
/// Wrappers of pointer and reference types are identical except for
/// their name -- they are formatted and accessed from a debugger
/// the same way.
///
/// Notice that "resolve_vmctx_memory_ptr" is external/builtin
/// subprogram that is not part of Wasm code.
fn replace_pointer_type<'a>(
    wrapper_die_id: write::UnitEntryId,
    parent_id: write::UnitEntryId,
    kind: WebAssemblyPtrKind,
    wasm_ptr_die_ref: write::DebugInfoRef,
    pointer_type_entry: &write::ConvertUnitEntry<Reader<'a>>,
    unit: &mut write::ConvertUnit<'_, Reader<'a>>,
) -> Result<(), Error> {
    const WASM_PTR_LEN: u8 = 4;

    macro_rules! add_tag {
        ($parent_id:ident, $tag:expr => $die:ident as $die_id:ident { $($a:path = $v:expr),* }) => {
            let $die_id = unit.unit.add($parent_id, $tag);
            #[allow(unused_variables, reason = "sometimes not used below")]
            let $die = unit.unit.get_mut($die_id);
            $( $die.set($a, $v); )*
        };
    }

    // Build DW_TAG_structure_type for the wrapper:
    //  .. DW_AT_name = "WebAssemblyPtrWrapper<T>",
    //  .. DW_AT_byte_size = 4,
    let name = match kind {
        WebAssemblyPtrKind::Pointer => format!(
            "WebAssemblyPtrWrapper<{}>",
            get_base_type_name(pointer_type_entry)?
        ),
        WebAssemblyPtrKind::Reference => format!(
            "WebAssemblyRefWrapper<{}>",
            get_base_type_name(pointer_type_entry)?
        ),
    };
    unit.unit
        .add_reserved(wrapper_die_id, parent_id, gimli::DW_TAG_structure_type);
    let wrapper_die = unit.unit.get_mut(wrapper_die_id);
    wrapper_die.set(
        gimli::DW_AT_name,
        write::AttributeValue::StringRef(unit.strings.add(name.as_str())),
    );
    wrapper_die.set(
        gimli::DW_AT_byte_size,
        write::AttributeValue::Data1(WASM_PTR_LEN),
    );

    // Build DW_TAG_pointer_type for `WebAssemblyPtrWrapper<T>*`:
    //  .. DW_AT_type = <wrapper_die>
    add_tag!(parent_id, gimli::DW_TAG_pointer_type => wrapper_ptr_type as wrapper_ptr_type_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wrapper_die_id)
    });

    let base_type = pointer_type_entry.attr_value(gimli::DW_AT_type);
    let base_type_id = if let Some(AttributeValue::UnitRef(offset)) = base_type {
        unit.convert_unit_ref(offset).ok()
    } else {
        None
    };

    // Build DW_TAG_reference_type for `T&`:
    //  .. DW_AT_type = <base_type>
    add_tag!(parent_id, gimli::DW_TAG_reference_type => ref_type as ref_type_id {});
    if let Some(base_type_id) = base_type_id {
        ref_type.set(
            gimli::DW_AT_type,
            write::AttributeValue::UnitRef(base_type_id),
        );
    }

    // Build DW_TAG_pointer_type for `T*`:
    //  .. DW_AT_type = <base_type>
    add_tag!(parent_id, gimli::DW_TAG_pointer_type => ptr_type as ptr_type_id {});
    if let Some(base_type_id) = base_type_id {
        ptr_type.set(
            gimli::DW_AT_type,
            write::AttributeValue::UnitRef(base_type_id),
        );
    }

    // Build wrapper_die's DW_TAG_template_type_parameter:
    //  .. DW_AT_name = "T"
    //  .. DW_AT_type = <base_type>
    add_tag!(wrapper_die_id, gimli::DW_TAG_template_type_parameter => t_param_die as t_param_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(unit.strings.add("T"))
    });
    if let Some(base_type_id) = base_type_id {
        t_param_die.set(
            gimli::DW_AT_type,
            write::AttributeValue::UnitRef(base_type_id),
        );
    }

    // Build wrapper_die's DW_TAG_member for `__ptr`:
    //  .. DW_AT_name = "__ptr"
    //  .. DW_AT_type = <wp_die>
    //  .. DW_AT_location = 0
    add_tag!(wrapper_die_id, gimli::DW_TAG_member => m_die as m_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(unit.strings.add("__ptr")),
        gimli::DW_AT_type = write::AttributeValue::DebugInfoRef(wasm_ptr_die_ref),
        gimli::DW_AT_data_member_location = write::AttributeValue::Data1(0)
    });

    // Build wrapper_die's DW_TAG_subprogram for `ptr()`:
    //  .. DW_AT_linkage_name = "wasmtime_resolve_vmctx_memory_ptr"
    //  .. DW_AT_name = "ptr"
    //  .. DW_AT_type = <ptr_type>
    //  .. DW_TAG_formal_parameter
    //  ..  .. DW_AT_type = <wrapper_ptr_type>
    //  ..  .. DW_AT_artificial = 1
    add_tag!(wrapper_die_id, gimli::DW_TAG_subprogram => deref_op_die as deref_op_die_id {
        gimli::DW_AT_linkage_name = write::AttributeValue::StringRef(unit.strings.add(versioned_stringify_ident!(wasmtime_resolve_vmctx_memory_ptr))),
        gimli::DW_AT_name = write::AttributeValue::StringRef(unit.strings.add("ptr")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(ptr_type_id)
    });
    add_tag!(deref_op_die_id, gimli::DW_TAG_formal_parameter => deref_op_this_param as deref_op_this_param_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wrapper_ptr_type_id),
        gimli::DW_AT_artificial = write::AttributeValue::Flag(true)
    });

    // Build wrapper_die's DW_TAG_subprogram for `operator*`:
    //  .. DW_AT_linkage_name = "wasmtime_resolve_vmctx_memory_ptr"
    //  .. DW_AT_name = "operator*"
    //  .. DW_AT_type = <ref_type>
    //  .. DW_TAG_formal_parameter
    //  ..  .. DW_AT_type = <wrapper_ptr_type>
    //  ..  .. DW_AT_artificial = 1
    add_tag!(wrapper_die_id, gimli::DW_TAG_subprogram => deref_op_die as deref_op_die_id {
        gimli::DW_AT_linkage_name = write::AttributeValue::StringRef(unit.strings.add(versioned_stringify_ident!(wasmtime_resolve_vmctx_memory_ptr))),
        gimli::DW_AT_name = write::AttributeValue::StringRef(unit.strings.add("operator*")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(ref_type_id)
    });
    add_tag!(deref_op_die_id, gimli::DW_TAG_formal_parameter => deref_op_this_param as deref_op_this_param_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wrapper_ptr_type_id),
        gimli::DW_AT_artificial = write::AttributeValue::Flag(true)
    });

    // Build wrapper_die's DW_TAG_subprogram for `operator->`:
    //  .. DW_AT_linkage_name = "wasmtime_resolve_vmctx_memory_ptr"
    //  .. DW_AT_name = "operator->"
    //  .. DW_AT_type = <ptr_type>
    //  .. DW_TAG_formal_parameter
    //  ..  .. DW_AT_type = <wrapper_ptr_type>
    //  ..  .. DW_AT_artificial = 1
    add_tag!(wrapper_die_id, gimli::DW_TAG_subprogram => deref_op_die as deref_op_die_id {
        gimli::DW_AT_linkage_name = write::AttributeValue::StringRef(unit.strings.add(versioned_stringify_ident!(wasmtime_resolve_vmctx_memory_ptr))),
        gimli::DW_AT_name = write::AttributeValue::StringRef(unit.strings.add("operator->")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(ptr_type_id)
    });
    add_tag!(deref_op_die_id, gimli::DW_TAG_formal_parameter => deref_op_this_param as deref_op_this_param_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wrapper_ptr_type_id),
        gimli::DW_AT_artificial = write::AttributeValue::Flag(true)
    });

    Ok(())
}

pub(crate) fn clone_unit<'a>(
    compilation: &mut Compilation<'_>,
    module: StaticModuleIndex,
    unit: &mut write::ConvertUnit<'_, Reader<'a>>,
    root_entry: &write::ConvertUnitEntry<Reader<'a>>,
    skeleton_root_entry: Option<&write::ConvertUnitEntry<Reader<'a>>>,
    addr_tr: &AddressTransform,
    out_module_synthetic_unit: &ModuleSyntheticUnit,
    translated: &mut HashSet<usize>,
    isa: &dyn TargetIsa,
) -> Result<(), Error> {
    let mut current_frame_base = InheritedAttr::new();
    let mut current_value_range = InheritedAttr::new();
    let mut current_scope_ranges = InheritedAttr::new();
    let mut current_subprogram = InheritedAttr::new();

    dbi_log!("Cloning CU {:?}", log_get_cu_summary(unit.read_unit));

    if let Some(convert_program) = unit.read_line_program(Some(unit.unit.encoding()), None)? {
        let (program, files) = clone_line_program(convert_program, addr_tr)?;
        unit.set_line_program(program, files);
    }

    if root_entry.tag == gimli::DW_TAG_compile_unit {
        log_begin_input_die(root_entry);

        let out_root_id = unit.unit.root();
        clone_die_attributes(
            unit,
            root_entry,
            addr_tr,
            None,
            out_root_id,
            None,
            None,
            EntryAttributesContext {
                subprograms: &mut current_subprogram,
                frame_base: None,
            },
            isa,
        )?;

        if let Some(skeleton_root_entry) = skeleton_root_entry {
            clone_die_attributes(
                unit,
                skeleton_root_entry,
                addr_tr,
                None,
                out_root_id,
                None,
                None,
                EntryAttributesContext {
                    subprograms: &mut current_subprogram,
                    frame_base: None,
                },
                isa,
            )?;
        }

        log_end_output_die(out_root_id, root_entry, unit);
    } else {
        // Can happen when the DWARF is split and we dont have the package/dwo files.
        // This is a better user experience than errorring.
        dbi_log!("... skipped: split DW_TAG_compile_unit entry missing");
        return Ok(()); // empty:
    }

    let mut entry = write::ConvertUnitEntry::null(unit.read_unit);
    while let Some(entry_id) = unit.read_entry(&mut entry)? {
        log_begin_input_die(&entry);
        let (Some(out_die_id), Some(parent)) = (entry_id, entry.parent) else {
            log_end_output_die_skipped(&entry, "unreachable");
            continue;
        };

        let new_stack_len = entry.depth;
        current_frame_base.update(new_stack_len);
        current_scope_ranges.update(new_stack_len);
        current_value_range.update(new_stack_len);
        current_subprogram.update(new_stack_len);
        let range_builder = if entry.tag == gimli::DW_TAG_subprogram {
            let range_builder = RangeInfoBuilder::from_subprogram_die(&entry, addr_tr)?;
            if let RangeInfoBuilder::Function(func) = range_builder {
                let frame_info = compilation.function_frame_info(module, func);
                current_value_range.push(new_stack_len, frame_info);
                let (symbol, _) = compilation.function(module, func);
                translated.insert(symbol);
                current_scope_ranges.push(new_stack_len, range_builder.get_ranges(addr_tr));
                Some(range_builder)
            } else {
                // FIXME current_scope_ranges.push()
                None
            }
        } else {
            if entry.has_attr(gimli::DW_AT_high_pc) || entry.has_attr(gimli::DW_AT_ranges) {
                let range_builder = RangeInfoBuilder::from(&entry)?;
                current_scope_ranges.push(new_stack_len, range_builder.get_ranges(addr_tr));
                Some(range_builder)
            } else {
                None
            }
        };

        if let Some(AttributeValue::Exprloc(expr)) = entry.attr_value(gimli::DW_AT_frame_base) {
            if let Some(expr) = compile_expression(&expr, unit.unit.encoding(), None)? {
                current_frame_base.push(new_stack_len, expr);
            }
        }

        if entry.tag == gimli::DW_TAG_pointer_type || entry.tag == gimli::DW_TAG_reference_type {
            // Wrap pointer types.
            let pointer_kind = match entry.tag {
                gimli::DW_TAG_pointer_type => WebAssemblyPtrKind::Pointer,
                gimli::DW_TAG_reference_type => WebAssemblyPtrKind::Reference,
                _ => panic!(),
            };
            replace_pointer_type(
                out_die_id,
                parent,
                pointer_kind,
                out_module_synthetic_unit.wasm_ptr_die_ref(),
                &entry,
                unit,
            )?;
            log_end_output_die(out_die_id, &entry, unit);
            continue;
        }

        unit.unit.add_reserved(out_die_id, parent, entry.tag);

        clone_die_attributes(
            unit,
            &entry,
            addr_tr,
            current_value_range.top(),
            out_die_id,
            range_builder,
            current_scope_ranges.top(),
            EntryAttributesContext {
                subprograms: &mut current_subprogram,
                frame_base: current_frame_base.top(),
            },
            isa,
        )?;

        // Data in WebAssembly memory always uses little-endian byte order.
        // If the native architecture is big-endian, we need to mark all
        // base types used to refer to WebAssembly memory as little-endian
        // using the DW_AT_endianity attribute, so that the debugger will
        // be able to correctly access them.
        if entry.tag == gimli::DW_TAG_base_type && isa.endianness() == Endianness::Big {
            let current_scope = unit.unit.get_mut(out_die_id);
            current_scope.set(
                gimli::DW_AT_endianity,
                write::AttributeValue::Endianity(gimli::DW_END_little),
            );
        }

        if entry.tag == gimli::DW_TAG_subprogram && !current_scope_ranges.is_empty() {
            append_vmctx_info(
                unit.unit,
                out_die_id,
                out_module_synthetic_unit.vmctx_ptr_die_ref(),
                addr_tr,
                current_value_range.top(),
                current_scope_ranges.top().context("range")?,
                unit.strings,
                isa,
            )?;
        }

        log_end_output_die(out_die_id, &entry, unit);
    }
    Ok(())
}
