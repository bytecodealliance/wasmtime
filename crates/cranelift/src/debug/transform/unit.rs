use super::address_transform::AddressTransform;
use super::attr::{clone_die_attributes, FileAttributeContext};
use super::expression::compile_expression;
use super::line_program::clone_line_program;
use super::range_info_builder::RangeInfoBuilder;
use super::refs::{PendingDebugInfoRefs, PendingUnitRefs, UnitRefsMap};
use super::utils::{add_internal_types, append_vmctx_info, get_function_frame_info};
use super::{DebugInputContext, Reader};
use crate::debug::ModuleMemoryOffset;
use crate::CompiledFunctionsMetadata;
use anyhow::{Context, Error};
use cranelift_codegen::ir::Endianness;
use cranelift_codegen::isa::TargetIsa;
use gimli::write;
use gimli::{AttributeValue, DebuggingInformationEntry, Unit};
use std::collections::HashSet;
use wasmtime_environ::DefinedFuncIndex;
use wasmtime_versioned_export_macros::versioned_stringify_ident;

struct InheritedAttr<T> {
    stack: Vec<(usize, T)>,
}

impl<T> InheritedAttr<T> {
    fn new() -> Self {
        InheritedAttr { stack: Vec::new() }
    }

    fn update(&mut self, depth: usize) {
        while !self.stack.is_empty() && self.stack.last().unwrap().0 >= depth {
            self.stack.pop();
        }
    }

    fn push(&mut self, depth: usize, value: T) {
        self.stack.push((depth, value));
    }

    fn top(&self) -> Option<&T> {
        self.stack.last().map(|entry| &entry.1)
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

fn get_base_type_name<R>(
    type_entry: &DebuggingInformationEntry<R>,
    unit: &Unit<R, R::Offset>,
    context: &DebugInputContext<R>,
) -> Result<String, Error>
where
    R: Reader,
{
    // FIXME remove recursion.
    if let Some(AttributeValue::UnitRef(ref offset)) = type_entry.attr_value(gimli::DW_AT_type)? {
        let mut entries = unit.entries_at_offset(*offset)?;
        entries.next_entry()?;
        if let Some(die) = entries.current() {
            if let Some(AttributeValue::DebugStrRef(str_offset)) =
                die.attr_value(gimli::DW_AT_name)?
            {
                return Ok(String::from(
                    context.debug_str.get_str(str_offset)?.to_string()?,
                ));
            }
            match die.tag() {
                gimli::DW_TAG_const_type => {
                    return Ok(format!("const {}", get_base_type_name(die, unit, context)?));
                }
                gimli::DW_TAG_pointer_type => {
                    return Ok(format!("{}*", get_base_type_name(die, unit, context)?));
                }
                gimli::DW_TAG_reference_type => {
                    return Ok(format!("{}&", get_base_type_name(die, unit, context)?));
                }
                gimli::DW_TAG_array_type => {
                    return Ok(format!("{}[]", get_base_type_name(die, unit, context)?));
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
fn replace_pointer_type<R>(
    parent_id: write::UnitEntryId,
    kind: WebAssemblyPtrKind,
    comp_unit: &mut write::Unit,
    wp_die_id: write::UnitEntryId,
    pointer_type_entry: &DebuggingInformationEntry<R>,
    unit: &Unit<R, R::Offset>,
    context: &DebugInputContext<R>,
    out_strings: &mut write::StringTable,
    pending_die_refs: &mut PendingUnitRefs,
) -> Result<write::UnitEntryId, Error>
where
    R: Reader,
{
    const WASM_PTR_LEN: u8 = 4;

    macro_rules! add_tag {
        ($parent_id:ident, $tag:expr => $die:ident as $die_id:ident { $($a:path = $v:expr),* }) => {
            let $die_id = comp_unit.add($parent_id, $tag);
            #[allow(unused_variables)]
            let $die = comp_unit.get_mut($die_id);
            $( $die.set($a, $v); )*
        };
    }

    // Build DW_TAG_structure_type for the wrapper:
    //  .. DW_AT_name = "WebAssemblyPtrWrapper<T>",
    //  .. DW_AT_byte_size = 4,
    let name = match kind {
        WebAssemblyPtrKind::Pointer => format!(
            "WebAssemblyPtrWrapper<{}>",
            get_base_type_name(pointer_type_entry, unit, context)?
        ),
        WebAssemblyPtrKind::Reference => format!(
            "WebAssemblyRefWrapper<{}>",
            get_base_type_name(pointer_type_entry, unit, context)?
        ),
    };
    add_tag!(parent_id, gimli::DW_TAG_structure_type => wrapper_die as wrapper_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add(name.as_str())),
        gimli::DW_AT_byte_size = write::AttributeValue::Data1(WASM_PTR_LEN)
    });

    // Build DW_TAG_pointer_type for `WebAssemblyPtrWrapper<T>*`:
    //  .. DW_AT_type = <wrapper_die>
    add_tag!(parent_id, gimli::DW_TAG_pointer_type => wrapper_ptr_type as wrapper_ptr_type_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wrapper_die_id)
    });

    let base_type_id = pointer_type_entry.attr_value(gimli::DW_AT_type)?;
    // Build DW_TAG_reference_type for `T&`:
    //  .. DW_AT_type = <base_type>
    add_tag!(parent_id, gimli::DW_TAG_reference_type => ref_type as ref_type_id {});
    if let Some(AttributeValue::UnitRef(ref offset)) = base_type_id {
        pending_die_refs.insert(ref_type_id, gimli::DW_AT_type, *offset);
    }

    // Build DW_TAG_pointer_type for `T*`:
    //  .. DW_AT_type = <base_type>
    add_tag!(parent_id, gimli::DW_TAG_pointer_type => ptr_type as ptr_type_id {});
    if let Some(AttributeValue::UnitRef(ref offset)) = base_type_id {
        pending_die_refs.insert(ptr_type_id, gimli::DW_AT_type, *offset);
    }

    // Build wrapper_die's DW_TAG_template_type_parameter:
    //  .. DW_AT_name = "T"
    //  .. DW_AT_type = <base_type>
    add_tag!(wrapper_die_id, gimli::DW_TAG_template_type_parameter => t_param_die as t_param_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("T"))
    });
    if let Some(AttributeValue::UnitRef(ref offset)) = base_type_id {
        pending_die_refs.insert(t_param_die_id, gimli::DW_AT_type, *offset);
    }

    // Build wrapper_die's DW_TAG_member for `__ptr`:
    //  .. DW_AT_name = "__ptr"
    //  .. DW_AT_type = <wp_die>
    //  .. DW_AT_location = 0
    add_tag!(wrapper_die_id, gimli::DW_TAG_member => m_die as m_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("__ptr")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wp_die_id),
        gimli::DW_AT_data_member_location = write::AttributeValue::Data1(0)
    });

    // Build wrapper_die's DW_TAG_subprogram for `ptr()`:
    //  .. DW_AT_linkage_name = "resolve_vmctx_memory_ptr"
    //  .. DW_AT_name = "ptr"
    //  .. DW_AT_type = <ptr_type>
    //  .. DW_TAG_formal_parameter
    //  ..  .. DW_AT_type = <wrapper_ptr_type>
    //  ..  .. DW_AT_artificial = 1
    add_tag!(wrapper_die_id, gimli::DW_TAG_subprogram => deref_op_die as deref_op_die_id {
        gimli::DW_AT_linkage_name = write::AttributeValue::StringRef(out_strings.add(versioned_stringify_ident!(resolve_vmctx_memory_ptr))),
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("ptr")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(ptr_type_id)
    });
    add_tag!(deref_op_die_id, gimli::DW_TAG_formal_parameter => deref_op_this_param as deref_op_this_param_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wrapper_ptr_type_id),
        gimli::DW_AT_artificial = write::AttributeValue::Flag(true)
    });

    // Build wrapper_die's DW_TAG_subprogram for `operator*`:
    //  .. DW_AT_linkage_name = "resolve_vmctx_memory_ptr"
    //  .. DW_AT_name = "operator*"
    //  .. DW_AT_type = <ref_type>
    //  .. DW_TAG_formal_parameter
    //  ..  .. DW_AT_type = <wrapper_ptr_type>
    //  ..  .. DW_AT_artificial = 1
    add_tag!(wrapper_die_id, gimli::DW_TAG_subprogram => deref_op_die as deref_op_die_id {
        gimli::DW_AT_linkage_name = write::AttributeValue::StringRef(out_strings.add(versioned_stringify_ident!(resolve_vmctx_memory_ptr))),
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("operator*")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(ref_type_id)
    });
    add_tag!(deref_op_die_id, gimli::DW_TAG_formal_parameter => deref_op_this_param as deref_op_this_param_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wrapper_ptr_type_id),
        gimli::DW_AT_artificial = write::AttributeValue::Flag(true)
    });

    // Build wrapper_die's DW_TAG_subprogram for `operator->`:
    //  .. DW_AT_linkage_name = "resolve_vmctx_memory_ptr"
    //  .. DW_AT_name = "operator->"
    //  .. DW_AT_type = <ptr_type>
    //  .. DW_TAG_formal_parameter
    //  ..  .. DW_AT_type = <wrapper_ptr_type>
    //  ..  .. DW_AT_artificial = 1
    add_tag!(wrapper_die_id, gimli::DW_TAG_subprogram => deref_op_die as deref_op_die_id {
        gimli::DW_AT_linkage_name = write::AttributeValue::StringRef(out_strings.add(versioned_stringify_ident!(resolve_vmctx_memory_ptr))),
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("operator->")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(ptr_type_id)
    });
    add_tag!(deref_op_die_id, gimli::DW_TAG_formal_parameter => deref_op_this_param as deref_op_this_param_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(wrapper_ptr_type_id),
        gimli::DW_AT_artificial = write::AttributeValue::Flag(true)
    });

    Ok(wrapper_die_id)
}

fn is_dead_code<R: Reader>(entry: &DebuggingInformationEntry<R>) -> bool {
    const TOMBSTONE: u64 = u32::MAX as u64;

    match entry.attr_value(gimli::DW_AT_low_pc) {
        Ok(Some(AttributeValue::Addr(addr))) => addr == TOMBSTONE,
        _ => false,
    }
}

pub(crate) fn clone_unit<'a, R>(
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
    split_unit: Option<&Unit<R, R::Offset>>,
    split_dwarf: Option<&gimli::Dwarf<R>>,
    context: &DebugInputContext<R>,
    addr_tr: &'a AddressTransform,
    funcs: &'a CompiledFunctionsMetadata,
    memory_offset: &ModuleMemoryOffset,
    out_encoding: gimli::Encoding,
    out_units: &mut write::UnitTable,
    out_strings: &mut write::StringTable,
    translated: &mut HashSet<DefinedFuncIndex>,
    isa: &dyn TargetIsa,
) -> Result<Option<(write::UnitId, UnitRefsMap, PendingDebugInfoRefs)>, Error>
where
    R: Reader,
{
    let mut die_ref_map = UnitRefsMap::new();
    let mut pending_die_refs = PendingUnitRefs::new();
    let mut pending_di_refs = PendingDebugInfoRefs::new();
    let mut stack = Vec::new();

    let mut program_unit = unit;
    let mut skeleton_die = None;

    // Get entries in outer scope to avoid borrowing on short lived temporary.
    let mut skeleton_entries = unit.entries();
    if let Some(unit) = split_unit {
        program_unit = unit;

        // From the spec, a skeleton unit has no children so we can assume the first, and only, entry is the DW_TAG_skeleton_unit (https://dwarfstd.org/doc/DWARF5.pdf).
        if let Some(die_tuple) = skeleton_entries.next_dfs()? {
            skeleton_die = Some(die_tuple.1);
        }
    }

    // Iterate over all of this compilation unit's entries.
    let mut entries = program_unit.entries();
    let (mut comp_unit, unit_id, file_map, file_index_base, cu_low_pc, wp_die_id, vmctx_die_id) =
        if let Some((depth_delta, entry)) = entries.next_dfs()? {
            assert_eq!(depth_delta, 0);
            let (out_line_program, debug_line_offset, file_map, file_index_base) =
                clone_line_program(
                    split_dwarf.unwrap_or(dwarf),
                    dwarf,
                    program_unit,
                    entry,
                    skeleton_die,
                    addr_tr,
                    out_encoding,
                    context.debug_str,
                    context.debug_line,
                    out_strings,
                )?;

            if entry.tag() == gimli::DW_TAG_compile_unit {
                let unit_id = out_units.add(write::Unit::new(out_encoding, out_line_program));
                let comp_unit = out_units.get_mut(unit_id);

                let root_id = comp_unit.root();
                die_ref_map.insert(entry.offset(), root_id);
                let cu_low_pc = unit.low_pc;

                clone_die_attributes(
                    split_dwarf.unwrap_or(dwarf),
                    &program_unit,
                    entry,
                    context,
                    addr_tr,
                    None,
                    comp_unit,
                    root_id,
                    None,
                    None,
                    cu_low_pc,
                    out_strings,
                    &mut pending_die_refs,
                    &mut pending_di_refs,
                    FileAttributeContext::Root(Some(debug_line_offset)),
                    isa,
                )?;

                let (wp_die_id, vmctx_die_id) =
                    add_internal_types(comp_unit, root_id, out_strings, memory_offset);

                stack.push(root_id);
                (
                    comp_unit,
                    unit_id,
                    file_map,
                    file_index_base,
                    cu_low_pc,
                    wp_die_id,
                    vmctx_die_id,
                )
            } else {
                // Can happen when the DWARF is split and we dont have the package/dwo files.
                // This is a better user experience than errorring.
                return Ok(None); // empty:
            }
        } else {
            return Ok(None); // empty
        };
    let mut skip_at_depth = None;
    let mut current_frame_base = InheritedAttr::new();
    let mut current_value_range = InheritedAttr::new();
    let mut current_scope_ranges = InheritedAttr::new();
    while let Some((depth_delta, entry)) = entries.next_dfs()? {
        // If `skip_at_depth` is `Some` then we previously decided to skip over
        // a node and all it's children. Let A be the last node processed, B be
        // the first node skipped, C be previous node, and D the current node.
        // Then `cached` is the difference from A to B, `depth` is the diffence
        // from B to C, and `depth_delta` is the differenc from C to D.
        let depth_delta = if let Some((depth, cached)) = skip_at_depth {
            // `new_depth` = B to D
            let new_depth = depth + depth_delta;
            // if D is below B continue to skip
            if new_depth > 0 {
                skip_at_depth = Some((new_depth, cached));
                continue;
            }
            // otherwise process D with `depth_delta` being the difference from A to D
            skip_at_depth = None;
            new_depth + cached
        } else {
            depth_delta
        };

        if !context
            .reachable
            .contains(&entry.offset().to_unit_section_offset(&unit))
            || is_dead_code(&entry)
        {
            // entry is not reachable: discarding all its info.
            // Here B = C so `depth` is 0. A is the previous node so `cached` =
            // `depth_delta`.
            skip_at_depth = Some((0, depth_delta));
            continue;
        }

        let new_stack_len = stack.len().wrapping_add(depth_delta as usize);
        current_frame_base.update(new_stack_len);
        current_scope_ranges.update(new_stack_len);
        current_value_range.update(new_stack_len);
        let range_builder = if entry.tag() == gimli::DW_TAG_subprogram {
            let range_builder = RangeInfoBuilder::from_subprogram_die(
                dwarf, &unit, entry, context, addr_tr, cu_low_pc,
            )?;
            if let RangeInfoBuilder::Function(func_index) = range_builder {
                if let Some(frame_info) = get_function_frame_info(memory_offset, funcs, func_index)
                {
                    current_value_range.push(new_stack_len, frame_info);
                }
                translated.insert(func_index);
                current_scope_ranges.push(new_stack_len, range_builder.get_ranges(addr_tr));
                Some(range_builder)
            } else {
                // FIXME current_scope_ranges.push()
                None
            }
        } else {
            let high_pc = entry.attr_value(gimli::DW_AT_high_pc)?;
            let ranges = entry.attr_value(gimli::DW_AT_ranges)?;
            if high_pc.is_some() || ranges.is_some() {
                let range_builder =
                    RangeInfoBuilder::from(dwarf, &unit, entry, context, cu_low_pc)?;
                current_scope_ranges.push(new_stack_len, range_builder.get_ranges(addr_tr));
                Some(range_builder)
            } else {
                None
            }
        };

        if depth_delta <= 0 {
            for _ in depth_delta..1 {
                stack.pop();
            }
        } else {
            assert_eq!(depth_delta, 1);
        }

        if let Some(AttributeValue::Exprloc(expr)) = entry.attr_value(gimli::DW_AT_frame_base)? {
            if let Some(expr) = compile_expression(&expr, unit.encoding(), None)? {
                current_frame_base.push(new_stack_len, expr);
            }
        }

        let parent = stack.last().unwrap();

        if entry.tag() == gimli::DW_TAG_pointer_type || entry.tag() == gimli::DW_TAG_reference_type
        {
            // Wrap pointer types.
            let pointer_kind = match entry.tag() {
                gimli::DW_TAG_pointer_type => WebAssemblyPtrKind::Pointer,
                gimli::DW_TAG_reference_type => WebAssemblyPtrKind::Reference,
                _ => panic!(),
            };
            let die_id = replace_pointer_type(
                *parent,
                pointer_kind,
                comp_unit,
                wp_die_id,
                entry,
                &unit,
                context,
                out_strings,
                &mut pending_die_refs,
            )?;
            stack.push(die_id);
            assert_eq!(stack.len(), new_stack_len);
            die_ref_map.insert(entry.offset(), die_id);
            continue;
        }

        let die_id = comp_unit.add(*parent, entry.tag());

        stack.push(die_id);
        assert_eq!(stack.len(), new_stack_len);
        die_ref_map.insert(entry.offset(), die_id);

        clone_die_attributes(
            split_dwarf.unwrap_or(dwarf),
            &unit,
            entry,
            context,
            addr_tr,
            current_value_range.top(),
            &mut comp_unit,
            die_id,
            range_builder,
            current_scope_ranges.top(),
            cu_low_pc,
            out_strings,
            &mut pending_die_refs,
            &mut pending_di_refs,
            FileAttributeContext::Children {
                file_map: &file_map,
                file_index_base,
                frame_base: current_frame_base.top(),
            },
            isa,
        )?;

        // Data in WebAssembly memory always uses little-endian byte order.
        // If the native architecture is big-endian, we need to mark all
        // base types used to refer to WebAssembly memory as little-endian
        // using the DW_AT_endianity attribute, so that the debugger will
        // be able to correctly access them.
        if entry.tag() == gimli::DW_TAG_base_type && isa.endianness() == Endianness::Big {
            let current_scope = comp_unit.get_mut(die_id);
            current_scope.set(
                gimli::DW_AT_endianity,
                write::AttributeValue::Endianity(gimli::DW_END_little),
            );
        }

        if entry.tag() == gimli::DW_TAG_subprogram && !current_scope_ranges.is_empty() {
            append_vmctx_info(
                comp_unit,
                die_id,
                vmctx_die_id,
                addr_tr,
                current_value_range.top(),
                current_scope_ranges.top().context("range")?,
                out_strings,
                isa,
            )?;
        }
    }
    die_ref_map.patch(pending_die_refs, comp_unit);
    Ok(Some((unit_id, die_ref_map, pending_di_refs)))
}
