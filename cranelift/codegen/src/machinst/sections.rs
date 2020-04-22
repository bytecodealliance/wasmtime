//! In-memory representation of compiled machine code, in multiple sections
//! (text, constant pool / rodata, etc). Emission occurs into multiple sections
//! simultaneously, so we buffer the result in memory and hand off to the
//! caller at the end of compilation.

use crate::binemit::{Addend, CodeOffset, CodeSink, Reloc};
use crate::ir::{ExternalName, Opcode, SourceLoc, TrapCode};

use alloc::vec::Vec;

/// A collection of sections with defined start-offsets.
pub struct MachSections {
    /// Sections, in offset order.
    pub sections: Vec<MachSection>,
}

impl MachSections {
    /// New, empty set of sections.
    pub fn new() -> MachSections {
        MachSections { sections: vec![] }
    }

    /// Add a section with a known offset and size. Returns the index.
    pub fn add_section(&mut self, start: CodeOffset, length: CodeOffset) -> usize {
        let idx = self.sections.len();
        self.sections.push(MachSection::new(start, length));
        idx
    }

    /// Mutably borrow the given section by index.
    pub fn get_section<'a>(&'a mut self, idx: usize) -> &'a mut MachSection {
        &mut self.sections[idx]
    }

    /// Get mutable borrows of two sections simultaneously. Used during
    /// instruction emission to provide references to the .text and .rodata
    /// (constant pool) sections.
    pub fn two_sections<'a>(
        &'a mut self,
        idx1: usize,
        idx2: usize,
    ) -> (&'a mut MachSection, &'a mut MachSection) {
        assert!(idx1 < idx2);
        assert!(idx1 < self.sections.len());
        assert!(idx2 < self.sections.len());
        let (first, rest) = self.sections.split_at_mut(idx2);
        (&mut first[idx1], &mut rest[0])
    }

    /// Emit this set of sections to a set of sinks for the code,
    /// relocations, traps, and stackmap.
    pub fn emit<CS: CodeSink>(&self, sink: &mut CS) {
        // N.B.: we emit every section into the .text section as far as
        // the `CodeSink` is concerned; we do not bother to segregate
        // the contents into the actual program text, the jumptable and the
        // rodata (constant pool). This allows us to generate code assuming
        // that these will not be relocated relative to each other, and avoids
        // having to designate each section as belonging in one of the three
        // fixed categories defined by `CodeSink`. If this becomes a problem
        // later (e.g. because of memory permissions or similar), we can
        // add this designation and segregate the output; take care, however,
        // to add the appropriate relocations in this case.

        for section in &self.sections {
            if section.data.len() > 0 {
                while sink.offset() < section.start_offset {
                    sink.put1(0);
                }
                section.emit(sink);
            }
        }
        sink.begin_jumptables();
        sink.begin_rodata();
        sink.end_codegen();
    }

    /// Get a list of source location mapping tuples in sorted-by-start-offset order.
    pub fn get_srclocs_sorted<'a>(&'a self) -> MachSectionsSrcLocs<'a> {
        MachSectionsSrcLocs::new(&self.sections)
    }

    /// Get the total required size for these sections.
    pub fn total_size(&self) -> CodeOffset {
        if self.sections.len() == 0 {
            0
        } else {
            // Find the last non-empty section.
            self.sections
                .iter()
                .rev()
                .find(|s| s.data.len() > 0)
                .map(|s| s.cur_offset_from_start())
                .unwrap_or(0)
        }
    }
}

/// An iterator over the srclocs in each section.
/// Returns MachSrcLocs in an order sorted by start location.
pub struct MachSectionsSrcLocs<'a> {
    sections: &'a [MachSection],
    cur_section: usize,
    cur_srcloc: usize,
    // For validation:
    last_offset: CodeOffset,
}

impl<'a> MachSectionsSrcLocs<'a> {
    fn new(sections: &'a [MachSection]) -> MachSectionsSrcLocs<'a> {
        MachSectionsSrcLocs {
            sections,
            cur_section: 0,
            cur_srcloc: 0,
            last_offset: 0,
        }
    }
}

impl<'a> Iterator for MachSectionsSrcLocs<'a> {
    type Item = &'a MachSrcLoc;

    fn next(&mut self) -> Option<&'a MachSrcLoc> {
        // We simply iterate through sections and srcloc records in order. This produces a
        // sorted order naturally because sections are in starting-offset-order, and srclocs
        // are produced as a section is emitted into, so are in order as well.

        // If we're out of sections, we're done.
        if self.cur_section >= self.sections.len() {
            return None;
        }

        // Otherwise, make sure we have a srcloc in the current section left to return, and
        // advance to the next section if not. Done if we run out of sections.
        while self.cur_srcloc >= self.sections[self.cur_section].srclocs.len() {
            self.cur_srcloc = 0;
            self.cur_section += 1;
            if self.cur_section >= self.sections.len() {
                return None;
            }
        }

        let loc = &self.sections[self.cur_section].srclocs[self.cur_srcloc];
        self.cur_srcloc += 1;
        debug_assert!(loc.start >= self.last_offset);
        self.last_offset = loc.start;
        Some(loc)
    }
}

/// An abstraction over MachSection and MachSectionSize: some
/// receiver of section data.
pub trait MachSectionOutput {
    /// Get the current offset from the start of all sections.
    fn cur_offset_from_start(&self) -> CodeOffset;

    /// Get the start offset of this section.
    fn start_offset(&self) -> CodeOffset;

    /// Add 1 byte to the section.
    fn put1(&mut self, _: u8);

    /// Add 2 bytes to the section.
    fn put2(&mut self, value: u16) {
        let [b0, b1] = value.to_le_bytes();
        self.put1(b0);
        self.put1(b1);
    }

    /// Add 4 bytes to the section.
    fn put4(&mut self, value: u32) {
        let [b0, b1, b2, b3] = value.to_le_bytes();
        self.put1(b0);
        self.put1(b1);
        self.put1(b2);
        self.put1(b3);
    }

    /// Add 8 bytes to the section.
    fn put8(&mut self, value: u64) {
        let [b0, b1, b2, b3, b4, b5, b6, b7] = value.to_le_bytes();
        self.put1(b0);
        self.put1(b1);
        self.put1(b2);
        self.put1(b3);
        self.put1(b4);
        self.put1(b5);
        self.put1(b6);
        self.put1(b7);
    }

    /// Add a slice of bytes to the section.
    fn put_data(&mut self, data: &[u8]);

    /// Add a relocation at the current offset.
    fn add_reloc(&mut self, loc: SourceLoc, kind: Reloc, name: &ExternalName, addend: Addend);

    /// Add a trap record at the current offset.
    fn add_trap(&mut self, loc: SourceLoc, code: TrapCode);

    /// Add a call return address record at the current offset.
    fn add_call_site(&mut self, loc: SourceLoc, opcode: Opcode);

    /// Start the output for the given source-location at the current offset.
    fn start_srcloc(&mut self, loc: SourceLoc);

    /// End the output for the previously-given source-location at the current offset.
    fn end_srcloc(&mut self);

    /// Align up to the given alignment.
    fn align_to(&mut self, align_to: CodeOffset) {
        assert!(align_to.is_power_of_two());
        while self.cur_offset_from_start() & (align_to - 1) != 0 {
            self.put1(0);
        }
    }
}

/// A section of output to be emitted to a CodeSink / RelocSink in bulk.
/// Multiple sections may be created with known start offsets in advance; the
/// usual use-case is to create the .text (code) and .rodata (constant pool) at
/// once, after computing the length of the code, so that constant references
/// can use known offsets as instructions are emitted.
pub struct MachSection {
    /// The starting offset of this section.
    pub start_offset: CodeOffset,
    /// The limit of this section, defined by the start of the next section.
    pub length_limit: CodeOffset,
    /// The section contents, as raw bytes.
    pub data: Vec<u8>,
    /// Any relocations referring to this section.
    pub relocs: Vec<MachReloc>,
    /// Any trap records referring to this section.
    pub traps: Vec<MachTrap>,
    /// Any call site records referring to this section.
    pub call_sites: Vec<MachCallSite>,
    /// Any source location mappings referring to this section.
    pub srclocs: Vec<MachSrcLoc>,
    /// The current source location in progress (after `start_srcloc()` and before `end_srcloc()`).
    /// This is a (start_offset, src_loc) tuple.
    pub cur_srcloc: Option<(CodeOffset, SourceLoc)>,
}

impl MachSection {
    /// Create a new section, known to start at `start_offset` and with a size limited to `length_limit`.
    pub fn new(start_offset: CodeOffset, length_limit: CodeOffset) -> MachSection {
        MachSection {
            start_offset,
            length_limit,
            data: vec![],
            relocs: vec![],
            traps: vec![],
            call_sites: vec![],
            srclocs: vec![],
            cur_srcloc: None,
        }
    }

    /// Emit this section to the CodeSink and other associated sinks.  The
    /// current offset of the CodeSink must match the starting offset of this
    /// section.
    pub fn emit<CS: CodeSink>(&self, sink: &mut CS) {
        assert!(sink.offset() == self.start_offset);

        let mut next_reloc = 0;
        let mut next_trap = 0;
        let mut next_call_site = 0;
        for (idx, byte) in self.data.iter().enumerate() {
            if next_reloc < self.relocs.len() {
                let reloc = &self.relocs[next_reloc];
                if reloc.offset == idx as CodeOffset {
                    sink.reloc_external(reloc.srcloc, reloc.kind, &reloc.name, reloc.addend);
                    next_reloc += 1;
                }
            }
            if next_trap < self.traps.len() {
                let trap = &self.traps[next_trap];
                if trap.offset == idx as CodeOffset {
                    sink.trap(trap.code, trap.srcloc);
                    next_trap += 1;
                }
            }
            if next_call_site < self.call_sites.len() {
                let call_site = &self.call_sites[next_call_site];
                if call_site.ret_addr == idx as CodeOffset {
                    sink.add_call_site(call_site.opcode, call_site.srcloc);
                    next_call_site += 1;
                }
            }
            sink.put1(*byte);
        }
    }
}

impl MachSectionOutput for MachSection {
    fn cur_offset_from_start(&self) -> CodeOffset {
        self.start_offset + self.data.len() as CodeOffset
    }

    fn start_offset(&self) -> CodeOffset {
        self.start_offset
    }

    fn put1(&mut self, value: u8) {
        assert!(((self.data.len() + 1) as CodeOffset) <= self.length_limit);
        self.data.push(value);
    }

    fn put_data(&mut self, data: &[u8]) {
        assert!(((self.data.len() + data.len()) as CodeOffset) <= self.length_limit);
        self.data.extend_from_slice(data);
    }

    fn add_reloc(&mut self, srcloc: SourceLoc, kind: Reloc, name: &ExternalName, addend: Addend) {
        let name = name.clone();
        self.relocs.push(MachReloc {
            offset: self.data.len() as CodeOffset,
            srcloc,
            kind,
            name,
            addend,
        });
    }

    fn add_trap(&mut self, srcloc: SourceLoc, code: TrapCode) {
        self.traps.push(MachTrap {
            offset: self.data.len() as CodeOffset,
            srcloc,
            code,
        });
    }

    fn add_call_site(&mut self, srcloc: SourceLoc, opcode: Opcode) {
        self.call_sites.push(MachCallSite {
            ret_addr: self.data.len() as CodeOffset,
            srcloc,
            opcode,
        });
    }

    fn start_srcloc(&mut self, loc: SourceLoc) {
        self.cur_srcloc = Some((self.cur_offset_from_start(), loc));
    }

    fn end_srcloc(&mut self) {
        let (start, loc) = self
            .cur_srcloc
            .take()
            .expect("end_srcloc() called without start_srcloc()");
        let end = self.cur_offset_from_start();
        // Skip zero-length extends.
        debug_assert!(end >= start);
        if end > start {
            self.srclocs.push(MachSrcLoc { start, end, loc });
        }
    }
}

/// A MachSectionOutput implementation that records only size.
pub struct MachSectionSize {
    /// The starting offset of this section.
    pub start_offset: CodeOffset,
    /// The current offset of this section.
    pub offset: CodeOffset,
}

impl MachSectionSize {
    /// Create a new size-counting dummy section.
    pub fn new(start_offset: CodeOffset) -> MachSectionSize {
        MachSectionSize {
            start_offset,
            offset: start_offset,
        }
    }

    /// Return the size this section would take if emitted with a real sink.
    pub fn size(&self) -> CodeOffset {
        self.offset - self.start_offset
    }
}

impl MachSectionOutput for MachSectionSize {
    fn cur_offset_from_start(&self) -> CodeOffset {
        // All size-counting sections conceptually start at offset 0; this doesn't
        // matter when counting code size.
        self.offset
    }

    fn start_offset(&self) -> CodeOffset {
        self.start_offset
    }

    fn put1(&mut self, _: u8) {
        self.offset += 1;
    }

    fn put_data(&mut self, data: &[u8]) {
        self.offset += data.len() as CodeOffset;
    }

    fn add_reloc(&mut self, _: SourceLoc, _: Reloc, _: &ExternalName, _: Addend) {}

    fn add_trap(&mut self, _: SourceLoc, _: TrapCode) {}

    fn add_call_site(&mut self, _: SourceLoc, _: Opcode) {}

    fn start_srcloc(&mut self, _: SourceLoc) {}

    fn end_srcloc(&mut self) {}
}

/// A relocation resulting from a compilation.
pub struct MachReloc {
    /// The offset at which the relocation applies, *relative to the
    /// containing section*.
    pub offset: CodeOffset,
    /// The original source location.
    pub srcloc: SourceLoc,
    /// The kind of relocation.
    pub kind: Reloc,
    /// The external symbol / name to which this relocation refers.
    pub name: ExternalName,
    /// The addend to add to the symbol value.
    pub addend: i64,
}

/// A trap record resulting from a compilation.
pub struct MachTrap {
    /// The offset at which the trap instruction occurs, *relative to the
    /// containing section*.
    pub offset: CodeOffset,
    /// The original source location.
    pub srcloc: SourceLoc,
    /// The trap code.
    pub code: TrapCode,
}

/// A call site record resulting from a compilation.
pub struct MachCallSite {
    /// The offset of the call's return address, *relative to the containing section*.
    pub ret_addr: CodeOffset,
    /// The original source location.
    pub srcloc: SourceLoc,
    /// The call's opcode.
    pub opcode: Opcode,
}

/// A source-location mapping resulting from a compilation.
#[derive(Clone, Debug)]
pub struct MachSrcLoc {
    /// The start of the region of code corresponding to a source location.
    /// This is relative to the start of the function, not to the start of the
    /// section.
    pub start: CodeOffset,
    /// The end of the region of code corresponding to a source location.
    /// This is relative to the start of the section, not to the start of the
    /// section.
    pub end: CodeOffset,
    /// The source location.
    pub loc: SourceLoc,
}
