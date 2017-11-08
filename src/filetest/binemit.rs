//! Test command for testing the binary machine code emission.
//!
//! The `binemit` test command generates binary machine code for every instruction in the input
//! functions and compares the results to the expected output.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use cretonne::binemit;
use cretonne::ir;
use cretonne::ir::entities::AnyEntity;
use cretonne::isa::TargetIsa;
use cretonne::regalloc::RegDiversions;
use cton_reader::TestCommand;
use filetest::subtest::{SubTest, Context, Result};
use utils::{match_directive, pretty_error};

struct TestBinEmit;

pub fn subtest(parsed: &TestCommand) -> Result<Box<SubTest>> {
    assert_eq!(parsed.command, "binemit");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestBinEmit))
    }
}

// Code sink that generates text.
struct TextSink {
    rnames: &'static [&'static str],
    offset: binemit::CodeOffset,
    text: String,
}

impl TextSink {
    /// Create a new empty TextSink.
    pub fn new(isa: &TargetIsa) -> Self {
        Self {
            rnames: isa.reloc_names(),
            offset: 0,
            text: String::new(),
        }
    }
}



impl binemit::CodeSink for TextSink {
    fn offset(&self) -> binemit::CodeOffset {
        self.offset
    }

    fn put1(&mut self, x: u8) {
        write!(self.text, "{:02x} ", x).unwrap();
        self.offset += 1;
    }

    fn put2(&mut self, x: u16) {
        write!(self.text, "{:04x} ", x).unwrap();
        self.offset += 2;
    }

    fn put4(&mut self, x: u32) {
        write!(self.text, "{:08x} ", x).unwrap();
        self.offset += 4;
    }

    fn put8(&mut self, x: u64) {
        write!(self.text, "{:016x} ", x).unwrap();
        self.offset += 8;
    }

    fn reloc_ebb(&mut self, reloc: binemit::Reloc, ebb_offset: binemit::CodeOffset) {
        write!(
            self.text,
            "{}({}) ",
            self.rnames[reloc.0 as usize],
            ebb_offset
        ).unwrap();
    }

    fn reloc_external(&mut self, reloc: binemit::Reloc, name: &ir::ExternalName) {
        write!(
            self.text,
            "{}({}) ",
            self.rnames[reloc.0 as usize],
            name,
        ).unwrap();
    }

    fn reloc_jt(&mut self, reloc: binemit::Reloc, jt: ir::JumpTable) {
        write!(self.text, "{}({}) ", self.rnames[reloc.0 as usize], jt).unwrap();
    }
}

impl SubTest for TestBinEmit {
    fn name(&self) -> Cow<str> {
        Cow::from("binemit")
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> Result<()> {
        let isa = context.isa.expect("binemit needs an ISA");
        let encinfo = isa.encoding_info();
        // TODO: Run a verifier pass over the code first to detect any bad encodings or missing/bad
        // value locations. The current error reporting is just crashing...
        let mut func = func.into_owned();

        // Fix the stack frame layout so we can test spill/fill encodings.
        let min_offset = func.stack_slots
            .keys()
            .map(|ss| func.stack_slots[ss].offset)
            .min();
        func.stack_slots.frame_size = min_offset.map(|off| (-off) as u32);

        let is_compressed = isa.flags().is_compressed();

        // Give an encoding to any instruction that doesn't already have one.
        let mut divert = RegDiversions::new();
        for ebb in func.layout.ebbs() {
            divert.clear();
            for inst in func.layout.ebb_insts(ebb) {
                if !func.encodings[inst].is_legal() {
                    let mut legal_encodings = isa.legal_encodings(
                        &func.dfg,
                        &func.dfg[inst],
                        func.dfg.ctrl_typevar(inst),
                    );

                    let enc = if is_compressed {
                        // Get the smallest legal encoding
                        legal_encodings
                            .filter(|e| {
                                let recipe_constraints = &encinfo.constraints[e.recipe()];
                                recipe_constraints.satisfied(inst, &divert, &func)
                            })
                            .min_by_key(|&e| encinfo.bytes(e))
                    } else {
                        // If not using compressed, just use the first encoding.
                        legal_encodings.next()
                    };

                    if let Some(enc) = enc {
                        func.encodings[inst] = enc;
                    }
                }
                divert.apply(&func.dfg[inst]);
            }
        }

        // Relax branches and compute EBB offsets based on the encodings.
        let code_size = binemit::relax_branches(&mut func, isa).map_err(|e| {
            pretty_error(&func, context.isa, e)
        })?;

        // Collect all of the 'bin:' directives on instructions.
        let mut bins = HashMap::new();
        for comment in &context.details.comments {
            if let Some(want) = match_directive(comment.text, "bin:") {
                match comment.entity {
                    AnyEntity::Inst(inst) => {
                        if let Some(prev) = bins.insert(inst, want) {
                            return Err(format!(
                                "multiple 'bin:' directives on {}: '{}' and '{}'",
                                func.dfg.display_inst(inst, isa),
                                prev,
                                want
                            ));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "'bin:' directive on non-inst {}: {}",
                            comment.entity,
                            comment.text
                        ))
                    }
                }
            }
        }
        if bins.is_empty() {
            return Err("No 'bin:' directives found".to_string());
        }

        // Now emit all instructions.
        let mut sink = TextSink::new(isa);
        for ebb in func.layout.ebbs() {
            divert.clear();
            // Correct header offsets should have been computed by `relax_branches()`.
            assert_eq!(
                sink.offset,
                func.offsets[ebb],
                "Inconsistent {} header offset",
                ebb
            );
            for inst in func.layout.ebb_insts(ebb) {
                sink.text.clear();
                let enc = func.encodings[inst];

                // Send legal encodings into the emitter.
                if enc.is_legal() {
                    let before = sink.offset;
                    isa.emit_inst(&func, inst, &mut divert, &mut sink);
                    let emitted = sink.offset - before;
                    // Verify the encoding recipe sizes against the ISAs emit_inst implementation.
                    assert_eq!(
                        emitted,
                        encinfo.bytes(enc),
                        "Inconsistent size for [{}] {}",
                        encinfo.display(enc),
                        func.dfg.display_inst(inst, isa)
                    );
                }

                // Check against bin: directives.
                if let Some(want) = bins.remove(&inst) {
                    if !enc.is_legal() {
                        return Err(format!(
                            "{} can't be encoded: {}",
                            inst,
                            func.dfg.display_inst(inst, isa)
                        ));
                    }
                    let have = sink.text.trim();
                    if have != want {
                        return Err(format!(
                            "Bad machine code for {}: {}\nWant: {}\nGot:  {}",
                            inst,
                            func.dfg.display_inst(inst, isa),
                            want,
                            have
                        ));
                    }
                }
            }
        }

        if sink.offset != code_size {
            return Err(format!(
                "Expected code size {}, got {}",
                code_size,
                sink.offset
            ));
        }

        Ok(())
    }
}
