//! Object file builder.
//!
//! Creates ELF image based on `Compilation` information. The ELF contains
//! functions and trampolines in the ".text" section. It also contains all
//! relocation records for the linking stage. If DWARF sections exist, their
//! content will be written as well.
//!
//! The object file has symbols for each function and trampoline, as well as
//! symbols that refer to libcalls.
//!
//! The function symbol names have format "_wasm_function_N", where N is
//! `FuncIndex`. The defined wasm function symbols refer to a JIT compiled
//! function body, the imported wasm function do not. The trampolines symbol
//! names have format "_trampoline_N", where N is `SignatureIndex`.

use crate::{CompiledFunction, RelocationTarget};
use anyhow::Result;
use cranelift_codegen::isa::{
    unwind::{systemv, UnwindInfo},
    TargetIsa,
};
use cranelift_codegen::TextSectionBuilder;
use gimli::write::{Address, EhFrame, EndianVec, FrameTable, Writer};
use gimli::RunTimeEndian;
use object::write::{Object, SectionId, StandardSegment, Symbol, SymbolId, SymbolSection};
use object::{Architecture, SectionKind, SymbolFlags, SymbolKind, SymbolScope};
use std::convert::TryFrom;
use std::ops::Range;
use wasmtime_environ::obj;
use wasmtime_environ::{DefinedFuncIndex, Module, PrimaryMap, SignatureIndex, Trampoline};

const TEXT_SECTION_NAME: &[u8] = b".text";

/// A helper structure used to assemble the final text section of an exectuable,
/// plus unwinding information and other related details.
///
/// This builder relies on Cranelift-specific internals but assembles into a
/// generic `Object` which will get further appended to in a compiler-agnostic
/// fashion later.
pub struct ModuleTextBuilder<'a> {
    /// The target that we're compiling for, used to query target-specific
    /// information as necessary.
    isa: &'a dyn TargetIsa,

    /// The object file that we're generating code into.
    obj: &'a mut Object<'static>,

    /// The WebAssembly module we're generating code for.
    module: &'a Module,

    text_section: SectionId,

    unwind_info: UnwindInfoBuilder<'a>,

    /// The corresponding symbol for each function, inserted as they're defined.
    ///
    /// If an index isn't here yet then it hasn't been defined yet.
    func_symbols: PrimaryMap<DefinedFuncIndex, SymbolId>,

    /// In-progress text section that we're using cranelift's `MachBuffer` to
    /// build to resolve relocations (calls) between functions.
    text: Box<dyn TextSectionBuilder>,
}

impl<'a> ModuleTextBuilder<'a> {
    pub fn new(obj: &'a mut Object<'static>, module: &'a Module, isa: &'a dyn TargetIsa) -> Self {
        // Entire code (functions and trampolines) will be placed
        // in the ".text" section.
        let text_section = obj.add_section(
            obj.segment_name(StandardSegment::Text).to_vec(),
            TEXT_SECTION_NAME.to_vec(),
            SectionKind::Text,
        );

        let num_defined = module.functions.len() - module.num_imported_funcs;
        Self {
            isa,
            obj,
            module,
            text_section,
            func_symbols: PrimaryMap::with_capacity(num_defined),
            unwind_info: Default::default(),
            text: isa.text_section_builder(num_defined as u32),
        }
    }

    /// Appends the `func` specified named `name` to this object.
    ///
    /// Returns the symbol associated with the function as well as the range
    /// that the function resides within the text section.
    pub fn append_func(
        &mut self,
        labeled: bool,
        name: Vec<u8>,
        func: &'a CompiledFunction,
    ) -> (SymbolId, Range<u64>) {
        let body_len = func.body.len() as u64;
        let off = self.text.append(labeled, &func.body, None);

        let symbol_id = self.obj.add_symbol(Symbol {
            name,
            value: off,
            size: body_len,
            kind: SymbolKind::Text,
            scope: SymbolScope::Compilation,
            weak: false,
            section: SymbolSection::Section(self.text_section),
            flags: SymbolFlags::None,
        });

        if let Some(info) = &func.unwind_info {
            self.unwind_info.push(off, body_len, info);
        }

        for r in func.relocations.iter() {
            match r.reloc_target {
                // Relocations against user-defined functions means that this is
                // a relocation against a module-local function, typically a
                // call between functions. The `text` field is given priority to
                // resolve this relocation before we actually emit an object
                // file, but if it can't handle it then we pass through the
                // relocation.
                RelocationTarget::UserFunc(index) => {
                    let defined_index = self.module.defined_func_index(index).unwrap();
                    if self.text.resolve_reloc(
                        off + u64::from(r.offset),
                        r.reloc,
                        r.addend,
                        defined_index.as_u32(),
                    ) {
                        continue;
                    }

                    // At this time it's expected that all relocations are
                    // handled by `text.resolve_reloc`, and anything that isn't
                    // handled is a bug in `text.resolve_reloc` or something
                    // transitively there. If truly necessary, though, then this
                    // loop could also be updated to forward the relocation to
                    // the final object file as well.
                    panic!(
                        "unresolved relocation could not be procesed against \
                         {index:?}: {r:?}"
                    );
                }

                // At this time it's not expected that any libcall relocations
                // are generated. Ideally we don't want relocations against
                // libcalls anyway as libcalls should go through indirect
                // `VMContext` tables to avoid needing to apply relocations at
                // module-load time as well.
                RelocationTarget::LibCall(call) => {
                    unimplemented!("cannot generate relocation against libcall {call:?}");
                }
            };
        }
        (symbol_id, off..off + body_len)
    }

    /// Appends a function to this object file.
    ///
    /// This is expected to be called in-order for ascending `index` values.
    pub fn func(&mut self, index: DefinedFuncIndex, func: &'a CompiledFunction) -> Range<u64> {
        let name = obj::func_symbol_name(self.module.func_index(index));
        let (symbol_id, range) = self.append_func(true, name.into_bytes(), func);
        assert_eq!(self.func_symbols.push(symbol_id), index);
        range
    }

    pub fn trampoline(&mut self, sig: SignatureIndex, func: &'a CompiledFunction) -> Trampoline {
        let name = obj::trampoline_symbol_name(sig);
        let (_, range) = self.append_func(false, name.into_bytes(), func);
        Trampoline {
            signature: sig,
            start: range.start,
            length: u32::try_from(range.end - range.start).unwrap(),
        }
    }

    /// Forces "veneers" to be used for inter-function calls in the text
    /// section which means that in-bounds optimized addresses are never used.
    ///
    /// This is only useful for debugging cranelift itself and typically this
    /// option is disabled.
    pub fn force_veneers(&mut self) {
        self.text.force_veneers();
    }

    /// Appends the specified amount of bytes of padding into the text section.
    ///
    /// This is only useful when fuzzing and/or debugging cranelift itself and
    /// for production scenarios `padding` is 0 and this function does nothing.
    pub fn append_padding(&mut self, padding: usize) {
        if padding == 0 {
            return;
        }
        self.text.append(false, &vec![0; padding], Some(1));
    }

    /// Indicates that the text section has been written completely and this
    /// will finish appending it to the original object.
    ///
    /// Note that this will also write out the unwind information sections if
    /// necessary.
    pub fn finish(mut self) -> Result<PrimaryMap<DefinedFuncIndex, SymbolId>> {
        // Finish up the text section now that we're done adding functions.
        let text = self.text.finish();
        self.obj
            .section_mut(self.text_section)
            .set_data(text, self.isa.code_section_alignment());

        // Append the unwind information for all our functions, if necessary.
        self.unwind_info
            .append_section(self.isa, self.obj, self.text_section);

        Ok(self.func_symbols)
    }
}

/// Builder used to create unwind information for a set of functions added to a
/// text section.
#[derive(Default)]
struct UnwindInfoBuilder<'a> {
    windows_xdata: Vec<u8>,
    windows_pdata: Vec<RUNTIME_FUNCTION>,
    systemv_unwind_info: Vec<(u64, &'a systemv::UnwindInfo)>,
}

// This is a mirror of `RUNTIME_FUNCTION` in the Windows API, but defined here
// to ensure everything is always `u32` and to have it available on all
// platforms. Note that all of these specifiers here are relative to a "base
// address" which we define as the base of where the text section is eventually
// loaded.
#[allow(non_camel_case_types)]
struct RUNTIME_FUNCTION {
    begin: u32,
    end: u32,
    unwind_address: u32,
}

impl<'a> UnwindInfoBuilder<'a> {
    /// Pushes the unwind information for a function into this builder.
    ///
    /// The function being described must be located at `function_offset` within
    /// the text section itself, and the function's size is specified by
    /// `function_len`.
    ///
    /// The `info` should come from Cranelift. and is handled here depending on
    /// its flavor.
    fn push(&mut self, function_offset: u64, function_len: u64, info: &'a UnwindInfo) {
        match info {
            // Windows unwind information is stored in two locations:
            //
            // * First is the actual unwinding information which is stored
            //   in the `.xdata` section. This is where `info`'s emitted
            //   information will go into.
            // * Second are pointers to connect all this unwind information,
            //   stored in the `.pdata` section. The `.pdata` section is an
            //   array of `RUNTIME_FUNCTION` structures.
            //
            // Due to how these will be loaded at runtime the `.pdata` isn't
            // actually assembled byte-wise here. Instead that's deferred to
            // happen later during `write_windows_unwind_info` which will apply
            // a further offset to `unwind_address`.
            UnwindInfo::WindowsX64(info) => {
                let unwind_size = info.emit_size();
                let mut unwind_info = vec![0; unwind_size];
                info.emit(&mut unwind_info);

                // `.xdata` entries are always 4-byte aligned
                //
                // FIXME: in theory we could "intern" the `unwind_info` value
                // here within the `.xdata` section. Most of our unwind
                // information for functions is probably pretty similar in which
                // case the `.xdata` could be quite small and `.pdata` could
                // have multiple functions point to the same unwinding
                // information.
                while self.windows_xdata.len() % 4 != 0 {
                    self.windows_xdata.push(0x00);
                }
                let unwind_address = self.windows_xdata.len();
                self.windows_xdata.extend_from_slice(&unwind_info);

                // Record a `RUNTIME_FUNCTION` which this will point to.
                self.windows_pdata.push(RUNTIME_FUNCTION {
                    begin: u32::try_from(function_offset).unwrap(),
                    end: u32::try_from(function_offset + function_len).unwrap(),
                    unwind_address: u32::try_from(unwind_address).unwrap(),
                });
            }

            // System-V is different enough that we just record the unwinding
            // information to get processed at a later time.
            UnwindInfo::SystemV(info) => {
                self.systemv_unwind_info.push((function_offset, info));
            }

            _ => panic!("some unwind info isn't handled here"),
        }
    }

    /// Appends the unwind information section, if any, to the `obj` specified.
    ///
    /// This function must be called immediately after the text section was
    /// added to a builder. The unwind information section must trail the text
    /// section immediately.
    ///
    /// The `text_section`'s section identifier is passed into this function.
    fn append_section(&self, isa: &dyn TargetIsa, obj: &mut Object<'_>, text_section: SectionId) {
        // This write will align the text section to a page boundary and then
        // return the offset at that point. This gives us the full size of the
        // text section at that point, after alignment.
        let text_section_size =
            obj.append_section_data(text_section, &[], isa.code_section_alignment());

        if self.windows_xdata.len() > 0 {
            assert!(self.systemv_unwind_info.len() == 0);
            // The `.xdata` section must come first to be just-after the `.text`
            // section for the reasons documented in `write_windows_unwind_info`
            // below.
            let segment = obj.segment_name(StandardSegment::Data).to_vec();
            let xdata_id = obj.add_section(segment, b".xdata".to_vec(), SectionKind::ReadOnlyData);
            let segment = obj.segment_name(StandardSegment::Data).to_vec();
            let pdata_id = obj.add_section(segment, b".pdata".to_vec(), SectionKind::ReadOnlyData);
            self.write_windows_unwind_info(obj, xdata_id, pdata_id, text_section_size);
        }

        if self.systemv_unwind_info.len() > 0 {
            let segment = obj.segment_name(StandardSegment::Data).to_vec();
            let section_id =
                obj.add_section(segment, b".eh_frame".to_vec(), SectionKind::ReadOnlyData);
            self.write_systemv_unwind_info(isa, obj, section_id, text_section_size)
        }
    }

    /// This function appends a nonstandard section to the object which is only
    /// used during `CodeMemory::publish`.
    ///
    /// This custom section effectively stores a `[RUNTIME_FUNCTION; N]` into
    /// the object file itself. This way registration of unwind info can simply
    /// pass this slice to the OS itself and there's no need to recalculate
    /// anything on the other end of loading a module from a precompiled object.
    ///
    /// Support for reading this is in `crates/jit/src/unwind/winx64.rs`.
    fn write_windows_unwind_info(
        &self,
        obj: &mut Object<'_>,
        xdata_id: SectionId,
        pdata_id: SectionId,
        text_section_size: u64,
    ) {
        // Currently the binary format supported here only supports
        // little-endian for x86_64, or at least that's all where it's tested.
        // This may need updates for other platforms.
        assert_eq!(obj.architecture(), Architecture::X86_64);

        // Append the `.xdata` section, or the actual unwinding information
        // codes and such which were built as we found unwind information for
        // functions.
        obj.append_section_data(xdata_id, &self.windows_xdata, 4);

        // Next append the `.pdata` section, or the array of `RUNTIME_FUNCTION`
        // structures stored in the binary.
        //
        // This memory will be passed at runtime to `RtlAddFunctionTable` which
        // takes a "base address" and the entries within `RUNTIME_FUNCTION` are
        // all relative to this base address. The base address we pass is the
        // address of the text section itself so all the pointers here must be
        // text-section-relative. The `begin` and `end` fields for the function
        // it describes are already text-section-relative, but the
        // `unwind_address` field needs to be updated here since the value
        // stored right now is `xdata`-section-relative. We know that the
        // `xdata` section follows the `.text` section so the
        // `text_section_size` is added in to calculate the final
        // `.text`-section-relative address of the unwind information.
        let mut pdata = Vec::with_capacity(self.windows_pdata.len() * 3 * 4);
        for info in self.windows_pdata.iter() {
            pdata.extend_from_slice(&info.begin.to_le_bytes());
            pdata.extend_from_slice(&info.end.to_le_bytes());
            let address = text_section_size + u64::from(info.unwind_address);
            let address = u32::try_from(address).unwrap();
            pdata.extend_from_slice(&address.to_le_bytes());
        }
        obj.append_section_data(pdata_id, &pdata, 4);
    }

    /// This function appends a nonstandard section to the object which is only
    /// used during `CodeMemory::publish`.
    ///
    /// This will generate a `.eh_frame` section, but not one that can be
    /// naively loaded. The goal of this section is that we can create the
    /// section once here and never again does it need to change. To describe
    /// dynamically loaded functions though each individual FDE needs to talk
    /// about the function's absolute address that it's referencing. Naturally
    /// we don't actually know the function's absolute address when we're
    /// creating an object here.
    ///
    /// To solve this problem the FDE address encoding mode is set to
    /// `DW_EH_PE_pcrel`. This means that the actual effective address that the
    /// FDE describes is a relative to the address of the FDE itself. By
    /// leveraging this relative-ness we can assume that the relative distance
    /// between the FDE and the function it describes is constant, which should
    /// allow us to generate an FDE ahead-of-time here.
    ///
    /// For now this assumes that all the code of functions will start at a
    /// page-aligned address when loaded into memory. The eh_frame encoded here
    /// then assumes that the text section is itself page aligned to its size
    /// and the eh_frame will follow just after the text section. This means
    /// that the relative offsets we're using here is the FDE going backwards
    /// into the text section itself.
    ///
    /// Note that the library we're using to create the FDEs, `gimli`, doesn't
    /// actually encode addresses relative to the FDE itself. Instead the
    /// addresses are encoded relative to the start of the `.eh_frame` section.
    /// This makes it much easier for us where we provide the relative offset
    /// from the start of `.eh_frame` to the function in the text section, which
    /// given our layout basically means the offset of the function in the text
    /// section from the end of the text section.
    ///
    /// A final note is that the reason we page-align the text section's size is
    /// so the .eh_frame lives on a separate page from the text section itself.
    /// This allows `.eh_frame` to have different virtual memory permissions,
    /// such as being purely read-only instead of read/execute like the code
    /// bits.
    fn write_systemv_unwind_info(
        &self,
        isa: &dyn TargetIsa,
        obj: &mut Object<'_>,
        section_id: SectionId,
        text_section_size: u64,
    ) {
        let mut cie = isa
            .create_systemv_cie()
            .expect("must be able to create a CIE for system-v unwind info");
        let mut table = FrameTable::default();
        cie.fde_address_encoding = gimli::constants::DW_EH_PE_pcrel;
        let cie_id = table.add_cie(cie);

        for (text_section_off, unwind_info) in self.systemv_unwind_info.iter() {
            let backwards_off = text_section_size - text_section_off;
            let actual_offset = -i64::try_from(backwards_off).unwrap();
            // Note that gimli wants an unsigned 64-bit integer here, but
            // unwinders just use this constant for a relative addition with the
            // address of the FDE, which means that the sign doesn't actually
            // matter.
            let fde = unwind_info.to_fde(Address::Constant(actual_offset as u64));
            table.add_fde(cie_id, fde);
        }
        let endian = match isa.triple().endianness().unwrap() {
            target_lexicon::Endianness::Little => RunTimeEndian::Little,
            target_lexicon::Endianness::Big => RunTimeEndian::Big,
        };
        let mut eh_frame = EhFrame(MyVec(EndianVec::new(endian)));
        table.write_eh_frame(&mut eh_frame).unwrap();

        // Some unwinding implementations expect a terminating "empty" length so
        // a 0 is written at the end of the table for those implementations.
        let mut endian_vec = (eh_frame.0).0;
        endian_vec.write_u32(0).unwrap();
        obj.append_section_data(section_id, endian_vec.slice(), 1);

        use gimli::constants;
        use gimli::write::Error;

        struct MyVec(EndianVec<RunTimeEndian>);

        impl Writer for MyVec {
            type Endian = RunTimeEndian;

            fn endian(&self) -> RunTimeEndian {
                self.0.endian()
            }

            fn len(&self) -> usize {
                self.0.len()
            }

            fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
                self.0.write(buf)
            }

            fn write_at(&mut self, pos: usize, buf: &[u8]) -> Result<(), Error> {
                self.0.write_at(pos, buf)
            }

            // FIXME(gimli-rs/gimli#576) this is the definition we want for
            // `write_eh_pointer` but the default implementation, at the time
            // of this writing, uses `offset - val` instead of `val - offset`.
            // A PR has been merged to fix this but until that's published we
            // can't use it.
            fn write_eh_pointer(
                &mut self,
                address: Address,
                eh_pe: constants::DwEhPe,
                size: u8,
            ) -> Result<(), Error> {
                let val = match address {
                    Address::Constant(val) => val,
                    Address::Symbol { .. } => unreachable!(),
                };
                assert_eq!(eh_pe.application(), constants::DW_EH_PE_pcrel);
                let offset = self.len() as u64;
                let val = val.wrapping_sub(offset);
                self.write_eh_pointer_data(val, eh_pe.format(), size)
            }
        }
    }
}
