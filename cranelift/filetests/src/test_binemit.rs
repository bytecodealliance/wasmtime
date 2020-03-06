//! Test command for testing the binary machine code emission.
//!
//! The `binemit` test command generates binary machine code for every instruction in the input
//! functions and compares the results to the expected output.

use crate::match_directive::match_directive;
use crate::subtest::{Context, SubTest, SubtestResult};
use cranelift_codegen::binemit::{self, CodeInfo, CodeSink, RegDiversions};
use cranelift_codegen::dbg::DisplayList;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir;
use cranelift_codegen::ir::entities::AnyEntity;
use cranelift_codegen::isa;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::settings::OptLevel;
use cranelift_reader::TestCommand;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;

struct TestBinEmit;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "binemit");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestBinEmit))
    }
}

/// Code sink that generates text.
struct TextSink {
    code_size: binemit::CodeOffset,
    offset: binemit::CodeOffset,
    text: String,
}

impl TextSink {
    /// Create a new empty TextSink.
    pub fn new() -> Self {
        Self {
            code_size: 0,
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

    fn reloc_block(&mut self, reloc: binemit::Reloc, block_offset: binemit::CodeOffset) {
        write!(self.text, "{}({}) ", reloc, block_offset).unwrap();
    }

    fn reloc_external(
        &mut self,
        reloc: binemit::Reloc,
        name: &ir::ExternalName,
        addend: binemit::Addend,
    ) {
        write!(self.text, "{}({}", reloc, name).unwrap();
        if addend != 0 {
            write!(self.text, "{:+}", addend).unwrap();
        }
        write!(self.text, ") ").unwrap();
    }

    fn reloc_constant(&mut self, reloc: binemit::Reloc, constant: ir::ConstantOffset) {
        write!(self.text, "{}({}) ", reloc, constant).unwrap();
    }

    fn reloc_jt(&mut self, reloc: binemit::Reloc, jt: ir::JumpTable) {
        write!(self.text, "{}({}) ", reloc, jt).unwrap();
    }

    fn trap(&mut self, code: ir::TrapCode, _srcloc: ir::SourceLoc) {
        write!(self.text, "{} ", code).unwrap();
    }

    fn begin_jumptables(&mut self) {
        self.code_size = self.offset
    }
    fn begin_rodata(&mut self) {}
    fn end_codegen(&mut self) {}
    fn add_stackmap(
        &mut self,
        _: &[ir::entities::Value],
        _: &ir::Function,
        _: &dyn isa::TargetIsa,
    ) {
    }
}

impl SubTest for TestBinEmit {
    fn name(&self) -> &'static str {
        "binemit"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> SubtestResult<()> {
        let isa = context.isa.expect("binemit needs an ISA");
        let encinfo = isa.encoding_info();
        // TODO: Run a verifier pass over the code first to detect any bad encodings or missing/bad
        // value locations. The current error reporting is just crashing...
        let mut func = func.into_owned();

        // Fix the stack frame layout so we can test spill/fill encodings.
        let min_offset = func
            .stack_slots
            .values()
            .map(|slot| slot.offset.unwrap())
            .min();
        func.stack_slots.layout_info = min_offset.map(|off| ir::StackLayoutInfo {
            frame_size: (-off) as u32,
            inbound_args_size: 0,
        });

        let opt_level = isa.flags().opt_level();

        // Give an encoding to any instruction that doesn't already have one.
        let mut divert = RegDiversions::new();
        for block in func.layout.blocks() {
            divert.clear();
            for inst in func.layout.block_insts(block) {
                if !func.encodings[inst].is_legal() {
                    // Find an encoding that satisfies both immediate field and register
                    // constraints.
                    if let Some(enc) = {
                        let mut legal_encodings = isa
                            .legal_encodings(&func, &func.dfg[inst], func.dfg.ctrl_typevar(inst))
                            .filter(|e| {
                                let recipe_constraints = &encinfo.constraints[e.recipe()];
                                recipe_constraints.satisfied(inst, &divert, &func)
                            });

                        if opt_level == OptLevel::SpeedAndSize {
                            // Get the smallest legal encoding
                            legal_encodings
                                .min_by_key(|&e| encinfo.byte_size(e, inst, &divert, &func))
                        } else {
                            // If not optimizing, just use the first encoding.
                            legal_encodings.next()
                        }
                    } {
                        func.encodings[inst] = enc;
                    }
                }
                divert.apply(&func.dfg[inst]);
            }
        }

        // Relax branches and compute block offsets based on the encodings.
        let mut cfg = ControlFlowGraph::with_function(&func);
        let mut domtree = DominatorTree::with_function(&func, &cfg);
        let CodeInfo { total_size, .. } =
            binemit::relax_branches(&mut func, &mut cfg, &mut domtree, isa)
                .map_err(|e| pretty_error(&func, context.isa, e))?;

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
                            comment.entity, comment.text
                        ));
                    }
                }
            }
        }
        if bins.is_empty() {
            return Err("No 'bin:' directives found".to_string());
        }

        // Now emit all instructions.
        let mut sink = TextSink::new();
        for block in func.layout.blocks() {
            divert.clear();
            // Correct header offsets should have been computed by `relax_branches()`.
            assert_eq!(
                sink.offset, func.offsets[block],
                "Inconsistent {} header offset",
                block
            );
            for (offset, inst, enc_bytes) in func.inst_offsets(block, &encinfo) {
                assert_eq!(sink.offset, offset);
                sink.text.clear();
                let enc = func.encodings[inst];

                // Send legal encodings into the emitter.
                if enc.is_legal() {
                    // Generate a better error message if output locations are not specified.
                    validate_location_annotations(&func, inst, isa, false)?;

                    let before = sink.offset;
                    isa.emit_inst(&func, inst, &mut divert, &mut sink);
                    let emitted = sink.offset - before;
                    // Verify the encoding recipe sizes against the ISAs emit_inst implementation.
                    assert_eq!(
                        emitted,
                        enc_bytes,
                        "Inconsistent size for [{}] {}",
                        encinfo.display(enc),
                        func.dfg.display_inst(inst, isa)
                    );
                }

                // Check against bin: directives.
                if let Some(want) = bins.remove(&inst) {
                    if !enc.is_legal() {
                        // A possible cause of an unencoded instruction is a missing location for
                        // one of the input/output operands.
                        validate_location_annotations(&func, inst, isa, true)?;
                        validate_location_annotations(&func, inst, isa, false)?;

                        // Do any encodings exist?
                        let encodings = isa
                            .legal_encodings(&func, &func.dfg[inst], func.dfg.ctrl_typevar(inst))
                            .map(|e| encinfo.display(e))
                            .collect::<Vec<_>>();

                        if encodings.is_empty() {
                            return Err(format!(
                                "No encodings found for: {}",
                                func.dfg.display_inst(inst, isa)
                            ));
                        }
                        return Err(format!(
                            "No matching encodings for {} in {}",
                            func.dfg.display_inst(inst, isa),
                            DisplayList(&encodings),
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

        sink.begin_jumptables();

        for (jt, jt_data) in func.jump_tables.iter() {
            let jt_offset = func.jt_offsets[jt];
            for block in jt_data.iter() {
                let rel_offset: i32 = func.offsets[*block] as i32 - jt_offset as i32;
                sink.put4(rel_offset as u32)
            }
        }

        sink.begin_rodata();

        // output constants
        for (_, constant_data) in func.dfg.constants.iter() {
            for byte in constant_data.iter() {
                sink.put1(*byte)
            }
        }

        sink.end_codegen();

        if sink.offset != total_size {
            return Err(format!(
                "Expected code size {}, got {}",
                total_size, sink.offset
            ));
        }

        Ok(())
    }
}

/// Validate registers/stack slots are correctly annotated.
fn validate_location_annotations(
    func: &ir::Function,
    inst: ir::Inst,
    isa: &dyn isa::TargetIsa,
    validate_inputs: bool,
) -> SubtestResult<()> {
    let values = if validate_inputs {
        func.dfg.inst_args(inst)
    } else {
        func.dfg.inst_results(inst)
    };

    if let Some(&v) = values.iter().find(|&&v| !func.locations[v].is_assigned()) {
        Err(format!(
            "Need register/stack slot annotation for {} in {}",
            v,
            func.dfg.display_inst(inst, isa)
        ))
    } else {
        Ok(())
    }
}
