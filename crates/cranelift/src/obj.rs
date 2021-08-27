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

use crate::debug::{DwarfSection, DwarfSectionRelocTarget};
use crate::{CompiledFunction, Relocation, RelocationTarget};
use anyhow::Result;
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::LibCall;
use cranelift_codegen::isa::{
    unwind::{systemv, UnwindInfo},
    TargetIsa,
};
use gimli::write::{Address, EhFrame, EndianVec, FrameTable, Writer};
use gimli::RunTimeEndian;
use object::write::{
    Object, Relocation as ObjectRelocation, SectionId, StandardSegment, Symbol, SymbolId,
    SymbolSection,
};
use object::{
    Architecture, RelocationEncoding, RelocationKind, SectionKind, SymbolFlags, SymbolKind,
    SymbolScope,
};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::mem;
use std::ops::Range;
use wasmtime_environ::obj;
use wasmtime_environ::{
    DefinedFuncIndex, EntityRef, FuncIndex, Module, PrimaryMap, SignatureIndex,
};

const TEXT_SECTION_NAME: &[u8] = b".text";

/// Iterates through all `LibCall` members and all runtime exported functions.
#[macro_export]
macro_rules! for_each_libcall {
    ($op:ident) => {
        $op![
            (UdivI64, wasmtime_i64_udiv),
            (UdivI64, wasmtime_i64_udiv),
            (SdivI64, wasmtime_i64_sdiv),
            (UremI64, wasmtime_i64_urem),
            (SremI64, wasmtime_i64_srem),
            (IshlI64, wasmtime_i64_ishl),
            (UshrI64, wasmtime_i64_ushr),
            (SshrI64, wasmtime_i64_sshr),
            (CeilF32, wasmtime_f32_ceil),
            (FloorF32, wasmtime_f32_floor),
            (TruncF32, wasmtime_f32_trunc),
            (NearestF32, wasmtime_f32_nearest),
            (CeilF64, wasmtime_f64_ceil),
            (FloorF64, wasmtime_f64_floor),
            (TruncF64, wasmtime_f64_trunc),
            (NearestF64, wasmtime_f64_nearest)
        ];
    };
}

fn write_libcall_symbols(obj: &mut Object) -> HashMap<LibCall, SymbolId> {
    let mut libcalls = HashMap::new();
    macro_rules! add_libcall_symbol {
        [$(($libcall:ident, $export:ident)),*] => {{
            $(
                let symbol_id = obj.add_symbol(Symbol {
                    name: stringify!($export).as_bytes().to_vec(),
                    value: 0,
                    size: 0,
                    kind: SymbolKind::Text,
                    scope: SymbolScope::Linkage,
                    weak: true,
                    section: SymbolSection::Undefined,
                    flags: SymbolFlags::None,
                });
                libcalls.insert(LibCall::$libcall, symbol_id);
            )+
        }};
    }
    for_each_libcall!(add_libcall_symbol);

    libcalls
}

/// A helper structure used to assemble the final text section of an exectuable,
/// plus unwinding information and other related details.
///
/// This builder relies on Cranelift-specific internals but assembles into a
/// generic `Object` which will get further appended to in a compiler-agnostic
/// fashion later.
pub struct ObjectBuilder<'a> {
    /// The target that we're compiling for, used to query target-specific
    /// information as necessary.
    isa: &'a dyn TargetIsa,

    /// The object file that we're generating code into.
    obj: &'a mut Object,

    /// The WebAssembly module we're generating code for.
    module: &'a Module,

    /// Map of injected symbols for all possible libcalls, used whenever there's
    /// a relocation against a libcall.
    libcalls: HashMap<LibCall, SymbolId>,

    /// Packed form of windows unwind tables which, if present, will get emitted
    /// to a windows-specific unwind info section.
    windows_unwind_info: Vec<RUNTIME_FUNCTION>,

    /// Pending unwinding information for DWARF-based platforms. This is used to
    /// build a `.eh_frame` lookalike at the very end of object building.
    systemv_unwind_info: Vec<(u64, &'a systemv::UnwindInfo)>,

    /// The corresponding symbol for each function, inserted as they're defined.
    ///
    /// If an index isn't here yet then it hasn't been defined yet.
    func_symbols: PrimaryMap<FuncIndex, SymbolId>,

    /// `object`-crate identifier for the text section.
    text_section: SectionId,

    /// The current offset within the `.text` section we're inserting at.
    ///
    /// Note that this does not match the text section offset in `obj`, but
    /// rather the cumulative size of `text_contents` below. This kept separate
    /// from `obj` because we edit the `text_contents` with relocations before
    /// emitting them to the object.
    current_text_off: u64,

    /// The segmented contents of the `.text` section, populated here as
    /// functions are inserted.
    ///
    /// Note that the text section isn't actually concatenated until the very
    /// end when after all relocations have been applied. Note that this is
    /// separate from the section in `obj` because these contents will be edited
    /// for relocations as necessary.
    ///
    /// The second element of the pair here is the desired alignment of the
    /// function body.
    text_contents: Vec<(Vec<u8>, u64)>,

    /// Map from text section offset, `u64`, to the index in `text_contents`
    /// where those contents live.
    text_locations: HashMap<u64, usize>,

    /// A list of relocations that must be resolved before the
    pending_relocs: Vec<PendingReloc<'a>>,

    /// Offset, after which, some relocs in `pending_relocs` will no longer be
    /// resolvable. This means that if code is added to the text section which
    /// would go beyond this point then a stub must be inserted to resolve at
    /// least one reloc.
    reloc_deadline: u64,

    /// Relocations within veneers which exceed the native platform's relative
    /// call instruction. These relocations will get filled in at the end
    /// with relative offsets to their target.
    relative_relocs: Vec<RelativeReloc>,

    /// A debug-only option to indicate that all inter-function calls should go
    /// through veneers, ideally testing the veneer emission code.
    pub force_jump_veneers: bool,
}

/// A pending relocation against a wasm-defined function that needs to be
/// resolved. These are collected as found in functions and then flushed via
/// `emit_jump_veneers` as necessary.
struct PendingReloc<'a> {
    /// The maximum offset in the text section this relocation can jump to.
    max_jump_distance: u64,
    /// The offset in the text section for the function that contains this
    /// relocation.
    offset: u64,
    /// The function that this relocation is against, or the target of the
    /// original function call.
    target: DefinedFuncIndex,
    /// The relocation entry from the compiled function, describing the
    /// relocation within the function at `offset`.
    reloc: &'a Relocation,
}

/// A relative relocation value within a veneer that will get resolved once all
/// functions have been emitted. One of these is inserted for each veneer
/// inserted and will get filled in in the `finish` method.
struct RelativeReloc {
    /// The offset in the text section to the veneer that contains this
    /// relocation.
    veneer_offset: u64,
    /// The offset, within the veneer, to where this relocation is located.
    reloc_offset: usize,
    /// The defined function this relocation is against. The relative distance
    /// from the location of the relocation to this function's definition will
    /// be written.
    target: DefinedFuncIndex,
    /// An optional addend for the relocation to add.
    addend: i64,
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

impl<'a> ObjectBuilder<'a> {
    pub fn new(obj: &'a mut Object, module: &'a Module, isa: &'a dyn TargetIsa) -> Self {
        // Entire code (functions and trampolines) will be placed
        // in the ".text" section.
        let text_section = obj.add_section(
            obj.segment_name(StandardSegment::Text).to_vec(),
            TEXT_SECTION_NAME.to_vec(),
            SectionKind::Text,
        );

        // Create symbols for imports -- needed during linking.
        let mut func_symbols = PrimaryMap::with_capacity(module.functions.len());
        for index in 0..module.num_imported_funcs {
            let symbol_id = obj.add_symbol(Symbol {
                name: obj::func_symbol_name(FuncIndex::new(index))
                    .as_bytes()
                    .to_vec(),
                value: 0,
                size: 0,
                kind: SymbolKind::Text,
                scope: SymbolScope::Linkage,
                weak: false,
                section: SymbolSection::Undefined,
                flags: SymbolFlags::None,
            });
            func_symbols.push(symbol_id);
        }

        let libcalls = write_libcall_symbols(obj);

        Self {
            isa,
            obj,
            module,
            text_section,
            func_symbols,
            libcalls,
            pending_relocs: Vec::new(),
            reloc_deadline: u64::MAX,
            windows_unwind_info: Vec::new(),
            systemv_unwind_info: Vec::new(),
            text_contents: Vec::new(),
            current_text_off: 0,
            text_locations: HashMap::new(),
            relative_relocs: Vec::new(),
            force_jump_veneers: false,
        }
    }

    /// Appends the `func` specified named `name` to this object.
    ///
    /// Returns the symbol associated with the function as well as the range
    /// that the function resides within the text section.
    fn append_func(
        &mut self,
        name: Vec<u8>,
        func: &'a CompiledFunction,
        body: Vec<u8>,
    ) -> (SymbolId, Range<u64>) {
        let body_len = body.len() as u64;
        let off = self.push_code(body, 1, true);

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

        match &func.unwind_info {
            // Windows unwind information is preferred to come after the code
            // itself. The information is appended here just after the function,
            // aligned to 4-bytes as required by Windows.
            //
            // The location of the unwind info, and the function it describes,
            // is then recorded in an unwind info table to get embedded into the
            // object at the end of compilation.
            Some(UnwindInfo::WindowsX64(info)) => {
                // Windows prefers Unwind info after the code -- writing it here.
                let unwind_size = info.emit_size();
                let mut unwind_info = vec![0; unwind_size];
                info.emit(&mut unwind_info);
                let unwind_off = self.push_code(unwind_info, 4, true);
                self.windows_unwind_info.push(RUNTIME_FUNCTION {
                    begin: u32::try_from(off).unwrap(),
                    end: u32::try_from(off + body_len).unwrap(),
                    unwind_address: u32::try_from(unwind_off).unwrap(),
                });
            }

            // System-V is different enough that we just record the unwinding
            // information to get processed at a later time.
            Some(UnwindInfo::SystemV(info)) => {
                self.systemv_unwind_info.push((off, info));
            }

            Some(_) => panic!("some unwind info isn't handled here"),
            None => {}
        }

        for r in func.relocations.iter() {
            let (symbol, symbol_offset) = match r.reloc_target {
                // Relocations against user-defined functions means that this is
                // a relocation against a module-local function. We want to
                // resolve these relocations ourselves and not actually leave
                // these to get resolve at load-time later. These relocations
                // are all relative as well so there's no need to resolve them
                // at load time when we can resolve them here at compile time.
                RelocationTarget::UserFunc(index) => {
                    let mut r = PendingReloc {
                        target: self.module.defined_func_index(index).unwrap(),
                        offset: off,
                        reloc: r,
                        max_jump_distance: 0,
                    };

                    // Determine this `reloc`'s deadline. This is the maximal
                    // forward distance for the relocation itself added to the
                    // position of the relocation, adjusted to assume that
                    // every pending relocation pessimistically needsd a
                    // veneer.
                    let offset_to_edit = r.offset + u64::from(r.reloc.offset);
                    r.max_jump_distance = offset_to_edit + r.limits().1;

                    self.enqueue_reloc(r);
                    continue;
                }

                // These relocations, unlike against user funcs above, typically
                // involve absolute addresses and need to get resolved at load
                // time. These are persisted immediately into the object file.
                //
                // FIXME: these, like user-defined-functions, should probably
                // use relative jumps and avoid absolute relocations. They don't
                // seem too common though so aren't necessarily that important
                // to optimize.
                RelocationTarget::LibCall(call) => (self.libcalls[&call], 0),
                RelocationTarget::JumpTable(jt) => (symbol_id, func.jt_offsets[jt]),
            };
            let (kind, encoding, size) = match r.reloc {
                Reloc::Abs4 => (RelocationKind::Absolute, RelocationEncoding::Generic, 32),
                Reloc::Abs8 => (RelocationKind::Absolute, RelocationEncoding::Generic, 64),

                // This is emitted by the old x86 backend and is only present
                // for when the constant rodata is separated from the code
                // itself. We don't do that, though, so we ignore these
                // relocations since the offsets already listed here are already
                // correct.
                Reloc::X86PCRelRodata4 => continue,

                other => unimplemented!("Unimplemented relocation {:?}", other),
            };
            self.obj
                .add_relocation(
                    self.text_section,
                    ObjectRelocation {
                        offset: off + r.offset as u64,
                        size,
                        kind,
                        encoding,
                        symbol,
                        addend: r.addend.wrapping_add(symbol_offset as i64),
                    },
                )
                .unwrap();
        }
        (symbol_id, off..off + body_len)
    }

    /// Inserts a chunk of code into the text section.
    ///
    /// This method is required to update various internal data structures
    /// about the structure of the text section. The most important part handled
    /// by this function is insertion of "jump veneers". If a previous
    /// function's relative call may reach further than the function being
    /// inserted, then a "veneer" is inserted to help it jump further, but the
    /// veneer needs to be inserted before `body` is inserted.
    fn push_code(&mut self, body: Vec<u8>, align: u64, allow_veneers: bool) -> u64 {
        let body_len = body.len() as u64;
        assert!(body_len > 0);

        // If this function would exceed the `reloc_deadline`, then all
        // relocations are processed to force some veneers to get generated.
        let mut ret = align_to(self.current_text_off, align);
        while ret + body_len >= self.reloc_deadline {
            assert!(allow_veneers);
            self.emit_jump_veneers(ret + body_len);
            ret = align_to(self.current_text_off, align);
        }

        // Once we can safely append the body we do so, updating various state
        // about how to get back to the body.
        self.text_locations.insert(ret, self.text_contents.len());
        self.current_text_off += body_len;
        self.text_contents.push((body, align));
        return ret;
    }

    /// Enqueues a `reloc` to get processed later.
    ///
    /// This function is used to push a `PendingReloc` record onto the list of
    /// relocs that need to get processed at some point in the future. This will
    /// update internal state about when to look at the relocs list and possibly
    /// emit jump veneers based on the relocation and the maximum distance that
    /// it can travel.
    fn enqueue_reloc(&mut self, reloc: PendingReloc<'a>) {
        // Decrease the deadline to emit veneers by pessimistically assuming
        // that this relocation will need a veneer.
        self.reloc_deadline -= max_jump_veneer_size(self.isa) as u64;

        // Insert the relocation into our pending list...
        let max = reloc.max_jump_distance;
        self.pending_relocs.push(reloc);

        // ... and finally possibly recalculate the relocation deadline based
        // on whether this relocation is shorter than some previous relocation.
        // Like above we pessimistically assume that all relocations need
        // veneers when calculating the deadline.
        let reloc_deadline =
            max - (self.pending_relocs.len() * max_jump_veneer_size(self.isa)) as u64;
        self.reloc_deadline = self.reloc_deadline.min(reloc_deadline);
    }

    /// Generates "jump veneers" as necessary, or otherwise handles and patches
    /// relative relocations.
    ///
    /// This function is the main workhorse of processing relative relocations
    /// desired within functions. These relative relocations are typically
    /// calls between wasm functions, but the distance on these calls is bounded
    /// based on the architecture in question. On the smaller end, for example,
    /// AArch64 calls can go at most 64MB either forwards or backwards. This
    /// function handles the case where two functions are more than 64MB apart,
    /// for example.
    ///
    /// Internally this function will process all entries in `pending_relocs`
    /// and handle them as necessary. For each pending relocation it can fall
    /// in a number of categories:
    ///
    /// * First the relocation could be against a known symbol, and the distance
    ///   from the relocation to the symbol is in-bounds. This means we can
    ///   simply patch the code directly and then we're good to go.
    ///
    /// * Second the relocation could be against a known symbol, but it could be
    ///   too far away to fit within the relocation's limits. This means that a
    ///   veneer is generated to jump to the destination.
    ///
    /// * Third a relocation could be against an unknown symbol, meaning it's a
    ///   function that hasn't been defined yet. If we're still within the jump
    ///   range, though, no action needs to be taken, and the relocation is
    ///   enqueued for processing later.
    ///
    /// * Finally a relocation against an unknown symbol may be so far away
    ///   that if the next symbol is inserted it couldn't reach its
    ///   destination. In this situation a veneer is generated.
    fn emit_jump_veneers(&mut self, forced_threshold: u64) {
        // Reset the relocation deadline since we're handling all relocations.
        // This will get updated in recursive calls to `enqueue_reloc` if
        // necessary.
        self.reloc_deadline = u64::MAX;

        let before = self.pending_relocs.len();
        for r in mem::take(&mut self.pending_relocs) {
            let target = self.module.func_index(r.target);
            match self.func_symbols.get(target) {
                Some(sym) => {
                    let sym_off = self.obj.symbol(*sym).value;
                    let distance = r.relative_distance_to(sym_off);
                    let (min_neg, max_pos) = r.limits();

                    // This is the second case described above. A relocation was
                    // added for a symbol but the symbol was defined very long
                    // ago. The only way to reach the symbol at this point
                    // is via a veneer.
                    if distance < min_neg || self.force_jump_veneers {
                        self.emit_jump_veneer(r);

                    // This case, if hit, represents a programming error. If
                    // the forward distance to the symbol is too large that
                    // means that this function wasn't called soon enough to
                    // insert a veneer, or something about the calculations of
                    // `forced_threshold` is wrong.
                    } else if distance > (max_pos as i64) {
                        panic!("should have emitted island earlier");

                    // This is the first case describe above. The distance to
                    // the symbol naturally fits within the limits of the
                    // relocation, so we can simply patch the distance in and
                    // the relocation is resolved.
                    } else {
                        self.patch_reloc(&r, distance);
                    }
                }

                // Function not defined, meaning it will come later. If the
                // reloc can't jump over the `forced_threshold` then we must
                // insert a veneer. Otherwise we can wait for the next batch
                // of veneers and continue to the next reloc.
                None => {
                    if forced_threshold < r.max_jump_distance {
                        self.enqueue_reloc(r);
                    } else {
                        self.emit_jump_veneer(r);
                    }
                }
            }
        }

        // At least one pending relocation should have been processed.
        assert!(
            self.pending_relocs.len() < before,
            "no relocations processed"
        );
    }

    /// Returns the `object`-crate's idea of endianness for the configured
    /// target.
    fn endian(&self) -> object::Endianness {
        match self.isa.triple().endianness().unwrap() {
            target_lexicon::Endianness::Little => object::Endianness::Little,
            target_lexicon::Endianness::Big => object::Endianness::Big,
        }
    }

    /// Patches a relocation with the `value` specified.
    ///
    /// This method is the implementation detail of actually modifying code
    /// emitted by Cranelift by patching in values to relocations. By doing
    /// this at object-assembly time here we can avoid doing this at load-time
    /// later, frontloading as much work as possible to make cache loads more
    /// efficient.
    fn patch_reloc(&mut self, r: &PendingReloc<'a>, value: i64) {
        type U32 = object::U32Bytes<object::Endianness>;
        type I32 = object::I32Bytes<object::Endianness>;

        // sanity-check
        let (min_neg, max_pos) = r.limits();
        assert!(min_neg <= value && value <= (max_pos as i64));

        let endian = self.endian();
        let code = self.text_locations[&r.offset];
        let code = &mut self.text_contents[code].0;

        match r.reloc.reloc {
            // This corresponds to the `R_AARCH64_CALL26` ELF relocation.
            Reloc::Arm64Call => {
                let reloc_address = reloc_address::<U32>(code, r.reloc.offset);
                let bits = (value as u32) >> 2;
                let insn = reloc_address.get(endian);
                let new_insn = (insn & 0xfc00_0000) | (bits & 0x03ff_ffff);
                reloc_address.set(endian, new_insn);
            }

            // This corresponds to the `R_386_PC32` ELF relocation.
            Reloc::X86CallPCRel4 => {
                reloc_address::<I32>(code, r.reloc.offset).set(endian, value as i32);
            }

            // This corresponds to the `R_390_PC32DBL` ELF relocation.
            Reloc::S390xPCRel32Dbl => {
                reloc_address::<I32>(code, r.reloc.offset).set(endian, (value as i32) >> 1);
            }

            other => panic!("unsupported function reloc {:?}", other),
        }
    }

    /// Emits a "jump veneer" at the current position in the code to resolve
    /// the relocation `r`.
    ///
    /// This function will ask the configured `TargetIsa` to generate a veneer
    /// for us and then that veneer will be inserted into the text section.
    /// Afterwards we update internal metadata to record that there's a relocation
    /// we need to update when all symbols are defined (since all veneers
    /// contain a 64-bit relative offset) and the original relocation needs to be
    /// patched to jump to the veneer we're synthesizing.
    fn emit_jump_veneer(&mut self, r: PendingReloc<'a>) {
        let (veneer, reloc_offset) = generate_jump_veneer(self.isa);
        let veneer_offset = self.push_code(veneer, 1, false);
        self.relative_relocs.push(RelativeReloc {
            veneer_offset,
            reloc_offset,
            target: r.target,
            addend: i64::from(r.reloc.addend),
        });
        self.patch_reloc(&r, r.relative_distance_to(veneer_offset));
    }

    /// Appends a function to this object file.
    ///
    /// This is expected to be called in-order for ascending `index` values.
    pub fn func(
        &mut self,
        index: DefinedFuncIndex,
        func: &'a CompiledFunction,
        code: Vec<u8>,
    ) -> Range<u64> {
        let index = self.module.func_index(index);
        let name = obj::func_symbol_name(index);
        let (symbol_id, range) = self.append_func(name.into_bytes(), func, code);
        assert_eq!(self.func_symbols.push(symbol_id), index);
        range
    }

    /// Helper function exclusively for tests to increase padding between
    /// functions to test the veneer insertion logic in this file.
    pub fn append_synthetic_padding(&mut self, amt: usize) {
        self.push_code(vec![0; amt], 1, true);
    }

    /// Inserts a compiled trampoline into this object.
    ///
    /// This is expected to be called after all the defined functions of a wasm
    /// file have been inserted.
    pub fn trampoline(&mut self, sig: SignatureIndex, func: &'a CompiledFunction, code: Vec<u8>) {
        let name = obj::trampoline_symbol_name(sig);
        self.append_func(name.into_bytes(), func, code);
    }

    pub fn align_text_to(&mut self, align: u64) {
        self.obj.append_section_data(self.text_section, &[], align);
    }

    pub fn dwarf_sections(&mut self, sections: &[DwarfSection]) -> Result<()> {
        // If we have DWARF data, write it in the object file.
        let (debug_bodies, debug_relocs): (Vec<_>, Vec<_>) = sections
            .iter()
            .map(|s| ((s.name, &s.body), (s.name, &s.relocs)))
            .unzip();
        let mut dwarf_sections_ids = HashMap::new();
        for (name, body) in debug_bodies {
            let segment = self.obj.segment_name(StandardSegment::Debug).to_vec();
            let section_id =
                self.obj
                    .add_section(segment, name.as_bytes().to_vec(), SectionKind::Debug);
            dwarf_sections_ids.insert(name, section_id);
            self.obj.append_section_data(section_id, &body, 1);
        }

        // Write all debug data relocations.
        for (name, relocs) in debug_relocs {
            let section_id = *dwarf_sections_ids.get(name).unwrap();
            for reloc in relocs {
                let target_symbol = match reloc.target {
                    DwarfSectionRelocTarget::Func(index) => {
                        self.func_symbols[self.module.func_index(DefinedFuncIndex::new(index))]
                    }
                    DwarfSectionRelocTarget::Section(name) => {
                        self.obj.section_symbol(dwarf_sections_ids[name])
                    }
                };
                self.obj.add_relocation(
                    section_id,
                    ObjectRelocation {
                        offset: u64::from(reloc.offset),
                        size: reloc.size << 3,
                        kind: RelocationKind::Absolute,
                        encoding: RelocationEncoding::Generic,
                        symbol: target_symbol,
                        addend: i64::from(reloc.addend),
                    },
                )?;
            }
        }

        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        // If there are any more pending relocations we have yet to resolve,
        // then we force them all to be resolved by setting the deadline for
        // emission at 0. No more functions are coming so now's the time to
        // flush all of them, if any.
        if !self.pending_relocs.is_empty() {
            self.emit_jump_veneers(0);
            assert!(self.pending_relocs.is_empty());
        }

        // Once we've handled all relocations between functions we can now
        // process all of the relocations which are required inside the veneers
        // themselves (if any). This is is only possible once the address of
        // all symbols are know, and here we're patching in the relative distance
        // between the immediate in the veneer and the destination it's
        // supposed to go to. This technique allows us to not actually generate
        // any relocations in the object file itself, since everything is
        // relative.
        let endian = self.endian();
        for reloc in mem::take(&mut self.relative_relocs) {
            // Calculate the actual value that will be stored in the relocation
            // location.
            let target = self.module.func_index(reloc.target);
            let symbol = self.func_symbols[target];
            let target_offset = self.obj.symbol(symbol).value;
            let reloc_offset = reloc.veneer_offset + (reloc.reloc_offset as u64);
            let value = target_offset
                .wrapping_add(reloc.addend as u64)
                .wrapping_sub(reloc_offset);

            // Store the `value` into the location specified in the relocation.
            let code = &mut self.text_contents[self.text_locations[&reloc.veneer_offset]].0;
            assert_eq!(self.isa.pointer_type().bits(), 64);
            reloc_address::<object::U64Bytes<object::Endianness>>(code, reloc.reloc_offset as u32)
                .set(endian, value);
        }

        // With relocations all handled we can shove everything into the final
        // text section now.
        for (i, (contents, align)) in self.text_contents.iter().enumerate() {
            let off = self
                .obj
                .append_section_data(self.text_section, contents, *align);
            debug_assert_eq!(self.text_locations[&off], i);
        }

        if self.windows_unwind_info.len() > 0 {
            self.append_windows_unwind_info();
        }
        if self.systemv_unwind_info.len() > 0 {
            self.append_systemv_unwind_info();
        }
        Ok(())
    }

    /// This function appends a nonstandard section to the object which is only
    /// used during `CodeMemory::allocate_for_object`.
    ///
    /// This custom section effectively stores a `[RUNTIME_FUNCTION; N]` into
    /// the object file itself. This way registration of unwind info can simply
    /// pass this slice to the OS itself and there's no need to recalculate
    /// anything on the other end of loading a module from a precompiled object.
    fn append_windows_unwind_info(&mut self) {
        // Currently the binary format supported here only supports
        // little-endian for x86_64, or at least that's all where it's tested.
        // This may need updates for other platforms.
        assert_eq!(self.obj.architecture(), Architecture::X86_64);

        // Page-align the text section so the unwind info can reside on a
        // separate page that doesn't need executable permissions.
        self.obj.append_section_data(self.text_section, &[], 0x1000);

        let segment = self.obj.segment_name(StandardSegment::Data).to_vec();
        let section_id = self.obj.add_section(
            segment,
            b"_wasmtime_winx64_unwind".to_vec(),
            SectionKind::ReadOnlyData,
        );
        let mut unwind_info = Vec::with_capacity(self.windows_unwind_info.len() * 3 * 4);
        for info in self.windows_unwind_info.iter() {
            unwind_info.extend_from_slice(&info.begin.to_le_bytes());
            unwind_info.extend_from_slice(&info.end.to_le_bytes());
            unwind_info.extend_from_slice(&info.unwind_address.to_le_bytes());
        }
        self.obj.append_section_data(section_id, &unwind_info, 1);
    }

    /// This function appends a nonstandard section to the object which is only
    /// used during `CodeMemory::allocate_for_object`.
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
    fn append_systemv_unwind_info(&mut self) {
        let segment = self.obj.segment_name(StandardSegment::Data).to_vec();
        let section_id = self.obj.add_section(
            segment,
            b"_wasmtime_eh_frame".to_vec(),
            SectionKind::ReadOnlyData,
        );
        let mut cie = self
            .isa
            .create_systemv_cie()
            .expect("must be able to create a CIE for system-v unwind info");
        let mut table = FrameTable::default();
        cie.fde_address_encoding = gimli::constants::DW_EH_PE_pcrel;
        let cie_id = table.add_cie(cie);

        // This write will align the text section to a page boundary (0x1000)
        // and then return the offset at that point. This gives us the full size
        // of the text section at that point, after alignment.
        let text_section_size = self.obj.append_section_data(self.text_section, &[], 0x1000);
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
        let endian = match self.isa.triple().endianness().unwrap() {
            target_lexicon::Endianness::Little => RunTimeEndian::Little,
            target_lexicon::Endianness::Big => RunTimeEndian::Big,
        };
        let mut eh_frame = EhFrame(MyVec(EndianVec::new(endian)));
        table.write_eh_frame(&mut eh_frame).unwrap();

        // Some unwinding implementations expect a terminating "empty" length so
        // a 0 is written at the end of the table for those implementations.
        let mut endian_vec = (eh_frame.0).0;
        endian_vec.write_u32(0).unwrap();
        self.obj
            .append_section_data(section_id, endian_vec.slice(), 1);

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

/// Align a size up to a power-of-two alignment.
fn align_to(x: u64, alignment: u64) -> u64 {
    let alignment_mask = alignment - 1;
    (x + alignment_mask) & !alignment_mask
}

impl PendingReloc<'_> {
    /// Returns the maximum negative offset and maximum positive offset that
    /// this relocation can reach.
    fn limits(&self) -> (i64, u64) {
        match self.reloc.reloc {
            Reloc::Arm64Call => (-(1 << 27), 1 << 27),
            Reloc::X86CallPCRel4 => (i32::MIN.into(), i32::MAX as u64),
            Reloc::S390xPCRel32Dbl => (i32::MIN.into(), i32::MAX as u64),

            other => panic!("unsupported function reloc {:?}", other),
        }
    }

    /// Returns the relative distance from this relocation to the `offset`
    /// specified in the text section.
    fn relative_distance_to(&self, offset: u64) -> i64 {
        (offset as i64)
            .wrapping_add(self.reloc.addend)
            .wrapping_sub((self.offset + u64::from(self.reloc.offset)) as i64)
    }
}

fn max_jump_veneer_size(isa: &dyn TargetIsa) -> usize {
    match isa.get_mach_backend() {
        Some(backend) => backend.max_jump_veneer_size(),
        // Old-style backends don't have veneers, and we'll panic if we need
        // them, so the size is zero.
        None => 0,
    }
}

fn generate_jump_veneer(isa: &dyn TargetIsa) -> (Vec<u8>, usize) {
    match isa.get_mach_backend() {
        Some(backend) => backend.generate_jump_veneer(),
        // This isn't implemented for the old backends, so we just don't
        // support objects of this size.
        None => panic!("jump veneers only supported on new backend"),
    }
}

/// Returns the address of `T` within `code` at `offset`, used to get the
/// address of where a relocation needs to be written.
fn reloc_address<T: object::Pod>(code: &mut [u8], offset: u32) -> &mut T {
    let (reloc, _rest) = usize::try_from(offset)
        .ok()
        .and_then(move |offset| code.get_mut(offset..))
        .and_then(|range| object::from_bytes_mut(range).ok())
        .expect("invalid reloc offset");
    reloc
}
