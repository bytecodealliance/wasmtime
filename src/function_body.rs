use backend::*;
use error::Error;
use module::FuncTyStore;
use wasmparser::{FunctionBody, Operator, Type};

// TODO: Use own declared `Type` enum.

/// Type of a control frame.
#[derive(Debug, Copy, Clone, PartialEq)]
enum ControlFrameKind {
    /// A regular block frame.
    ///
    /// Can be used for an implicit function block.
    Block { end_label: Label },
    /// Loop frame (branching to the beginning of block).
    Loop { header: Label },
    /// True-subblock of if expression.
    IfTrue {
        /// If jump happens inside the if-true block then control will
        /// land on this label.
        end_label: Label,

        /// If the condition of the `if` statement is unsatisfied, control
        /// will land on this label. This label might point to `else` block if it
        /// exists. Otherwise it equal to `end_label`.
        if_not: Label,
    },
    /// False-subblock of if expression.
    IfFalse { end_label: Label },
}

impl ControlFrameKind {
    /// Returns a label which should be used as a branch destination.
    fn block_end(&self) -> Option<Label> {
        match *self {
            ControlFrameKind::Block { end_label } => Some(end_label),
            ControlFrameKind::IfTrue { end_label, .. } => Some(end_label),
            ControlFrameKind::IfFalse { end_label } => Some(end_label),
            ControlFrameKind::Loop { .. } => None,
        }
    }

    fn end_labels(&self) -> impl Iterator<Item = Label> {
        self.block_end()
            .into_iter()
            .chain(if let ControlFrameKind::IfTrue { if_not, .. } = self {
                // this is `if .. end` construction. Define the `if_not` label.
                Some(*if_not)
            } else {
                None
            })
    }

    fn is_loop(&self) -> bool {
        match *self {
            ControlFrameKind::Loop { .. } => true,
            _ => false,
        }
    }

    fn branch_target(&self) -> Label {
        match *self {
            ControlFrameKind::Block { end_label } => end_label,
            ControlFrameKind::IfTrue { end_label, .. } => end_label,
            ControlFrameKind::IfFalse { end_label } => end_label,
            ControlFrameKind::Loop { header } => header,
        }
    }
}

struct ControlFrame {
    kind: ControlFrameKind,
    /// Boolean which signals whether value stack became polymorphic. Value stack starts in non-polymorphic state and
    /// becomes polymorphic only after an instruction that never passes control further is executed,
    /// i.e. `unreachable`, `br` (but not `br_if`!), etc.
    unreachable: bool,
    /// State specific to the block (free temp registers, stack etc) which should be replaced
    /// at the end of the block
    block_state: BlockState,
    ty: Type,
}

fn arity(ty: Type) -> u32 {
    if ty == Type::EmptyBlockType {
        0
    } else {
        1
    }
}

impl ControlFrame {
    pub fn new(kind: ControlFrameKind, block_state: BlockState, ty: Type) -> ControlFrame {
        ControlFrame {
            kind,
            block_state,
            ty,
            unreachable: false,
        }
    }

    pub fn arity(&self) -> u32 {
        arity(self.ty)
    }

    /// Marks this control frame as reached stack-polymorphic state.
    pub fn mark_unreachable(&mut self) {
        self.unreachable = true;
    }
}

pub fn translate(
    session: &mut CodeGenSession,
    translation_ctx: &FuncTyStore,
    func_idx: u32,
    body: &FunctionBody,
) -> Result<(), Error> {
    fn break_from_control_frame_with_id(
        ctx: &mut Context,
        control_frames: &mut Vec<ControlFrame>,
        idx: usize,
    ) {
        let control_frame = control_frames.get_mut(idx).expect("wrong depth");

        if control_frame.kind.is_loop() {
            ctx.restore_locals_to(&control_frame.block_state.locals);
        } else {
            // We can't do any execution after the function end so we just skip this logic
            // if we're breaking out of the whole function.
            if idx != 0 {
                // Workaround for borrowck limitations
                let should_set = if let Some(locals) = control_frame.block_state.end_locals.as_ref()
                {
                    ctx.restore_locals_to(locals);
                    false
                } else {
                    true
                };

                if should_set {
                    control_frame.block_state.end_locals = Some(ctx.block_state.locals.clone());
                }
            }

            ctx.return_from_block(control_frame.arity());
        }

        ctx.br(control_frame.kind.branch_target());
    }

    let locals = body.get_locals_reader()?;

    let func_type = translation_ctx.func_type(func_idx);
    let arg_count = func_type.params.len() as u32;
    let return_ty = if func_type.returns.len() == 1 {
        func_type.returns[0]
    } else if func_type.returns.len() == 0 {
        Type::EmptyBlockType
    } else {
        panic!("We don't support multiple returns yet");
    };

    let mut num_locals = 0;
    for local in locals {
        let (count, _ty) = local?;
        num_locals += count;
    }

    let ctx = &mut session.new_context(func_idx);
    let operators = body.get_operators_reader()?;

    // We must add 1 here to supply `vmctx`
    let func = ctx.start_function(arg_count, num_locals);

    let mut control_frames = Vec::new();

    // Upon entering the function implicit frame for function body is pushed. It has the same
    // result type as the function itself. Branching to it is equivalent to returning from the function.
    let epilogue_label = ctx.create_label();
    let function_block_state = ctx.start_block();
    control_frames.push(ControlFrame::new(
        ControlFrameKind::Block {
            end_label: epilogue_label,
        },
        function_block_state,
        return_ty,
    ));

    let mut operators = itertools::put_back(operators.into_iter());

    // TODO: We want to make this a state machine (maybe requires 1-element lookahead? Not sure) so that we
    //       can coelesce multiple `end`s and optimise break-at-end-of-block into noop.
    // TODO: Does coelescing multiple `end`s matter since at worst this really only elides a single move at
    //       the end of a function, and this is probably a no-op anyway due to register renaming.
    loop {
        let op = if let Some(op) = operators.next() {
            op?
        } else {
            break;
        };

        match op {
            Operator::End | Operator::Else => {}
            _ => {
                if control_frames
                    .last()
                    .expect("Control stack never empty")
                    .unreachable
                {
                    continue;
                }
            }
        }

        match op {
            Operator::Unreachable => {
                control_frames
                    .last_mut()
                    .expect("control stack is never empty")
                    .mark_unreachable();
                ctx.trap();
            }
            Operator::Block { ty } => {
                let label = ctx.create_label();
                let state = ctx.start_block();
                control_frames.push(ControlFrame::new(
                    ControlFrameKind::Block { end_label: label },
                    state,
                    ty,
                ));
            }
            Operator::Return => {
                control_frames
                    .last_mut()
                    .expect("Control stack is empty!")
                    .mark_unreachable();

                break_from_control_frame_with_id(ctx, &mut control_frames, 0);
            }
            Operator::Br { relative_depth } => {
                control_frames
                    .last_mut()
                    .expect("Control stack is empty!")
                    .mark_unreachable();

                let idx = control_frames.len() - 1 - relative_depth as usize;

                break_from_control_frame_with_id(ctx, &mut control_frames, idx);
            }
            Operator::BrIf { relative_depth } => {
                let idx = control_frames.len() - 1 - relative_depth as usize;

                let if_not = ctx.create_label();

                ctx.jump_if_false(if_not);

                break_from_control_frame_with_id(ctx, &mut control_frames, idx);

                ctx.define_label(if_not);
            }
            Operator::If { ty } => {
                let end_label = ctx.create_label();
                let if_not = ctx.create_label();

                ctx.jump_if_false(if_not);
                let state = ctx.start_block();

                control_frames.push(ControlFrame::new(
                    ControlFrameKind::IfTrue { end_label, if_not },
                    state,
                    ty,
                ));
            }
            Operator::Loop { ty } => {
                let header = ctx.create_label();

                ctx.define_label(header);
                let state = ctx.start_block();

                control_frames.push(ControlFrame::new(
                    ControlFrameKind::Loop { header },
                    state,
                    ty,
                ));
            }
            Operator::Else => {
                match control_frames.pop() {
                    Some(ControlFrame {
                        kind: ControlFrameKind::IfTrue { if_not, end_label },
                        ty,
                        block_state,
                        ..
                    }) => {
                        ctx.return_from_block(arity(ty));
                        ctx.reset_block(block_state.clone());

                        // Finalize `then` block by jumping to the `end_label`.
                        ctx.br(end_label);

                        // Define `if_not` label here, so if the corresponding `if` block receives
                        // 0 it will branch here.
                        // After that reset stack depth to the value before entering `if` block.
                        ctx.define_label(if_not);

                        // Carry over the `end_label`, so it will be resolved when the corresponding `end`
                        // is encountered.
                        //
                        // Also note that we reset `stack_depth` to the value before entering `if` block.
                        let mut frame = ControlFrame::new(
                            ControlFrameKind::IfFalse { end_label },
                            block_state,
                            ty,
                        );
                        control_frames.push(frame);
                    }
                    Some(_) => panic!("else expects if block"),
                    None => panic!("control stack is never empty"),
                };
            }
            Operator::End => {
                // TODO: Merge `End`s so that we can
                //       A) Move values directly into RAX when returning from deeply-nested blocks.
                //       B) Avoid restoring locals when not necessary.
                //
                //       This doesn't require lookahead but it does require turning this loop into
                //       a kind of state machine.
                let mut control_frame = control_frames.pop().expect("control stack is never empty");
                let mut labels = control_frame
                    .kind
                    .end_labels()
                    .collect::<Vec<_>>();

                let mut end = control_frame.block_state.end_locals.take();

                // Fold `End`s together to prevent unnecessary shuffling of locals
                loop {
                    let op = if let Some(op) = operators.next() {
                        op?
                    } else {
                        break;
                    };

                    match op {
                        Operator::End => {
                            control_frame =
                                control_frames.pop().expect("control stack is never empty");

                            labels.extend(control_frame.kind.end_labels());

                            end = control_frame.block_state.end_locals.take().or(end);
                        }
                        other => {
                            operators.put_back(Ok(other));
                            break;
                        }
                    }
                }

                let arity = control_frame.arity();

                // Don't bother generating this code if we're in unreachable code
                if !control_frame.unreachable {
                    ctx.return_from_block(arity);

                    // If there are no remaining frames we've hit the end of the function - we don't need to
                    // restore locals since no execution will happen after this point.
                    if !control_frames.is_empty() {
                        if let Some(end) = end {
                            ctx.restore_locals_to(&end);
                        }
                    }
                }

                // TODO: What is the correct order of this and the `define_label`? It's clear for `block`s
                //       but I'm not certain for `if..then..else..end`.
                ctx.end_block(control_frame.block_state, |ctx| {
                    for label in labels {
                        ctx.define_label(label);
                    }
                });
            }
            Operator::I32Eq => ctx.i32_eq(),
            Operator::I32Eqz => ctx.i32_eqz(),
            Operator::I32Ne => ctx.i32_neq(),
            Operator::I32LtS => ctx.i32_lt_s(),
            Operator::I32LeS => ctx.i32_le_s(),
            Operator::I32GtS => ctx.i32_gt_s(),
            Operator::I32GeS => ctx.i32_ge_s(),
            Operator::I32LtU => ctx.i32_lt_u(),
            Operator::I32LeU => ctx.i32_le_u(),
            Operator::I32GtU => ctx.i32_gt_u(),
            Operator::I32GeU => ctx.i32_ge_u(),
            Operator::I32Add => ctx.i32_add(),
            Operator::I32Sub => ctx.i32_sub(),
            Operator::I32And => ctx.i32_and(),
            Operator::I32Or => ctx.i32_or(),
            Operator::I32Xor => ctx.i32_xor(),
            Operator::I32Mul => ctx.i32_mul(),
            Operator::I32Clz => ctx.i32_clz(),
            Operator::I32Ctz => ctx.i32_ctz(),
            Operator::I32Popcnt => ctx.i32_popcnt(),
            Operator::I64Eq => ctx.i64_eq(),
            Operator::I64Eqz => ctx.i64_eqz(),
            Operator::I64Ne => ctx.i64_neq(),
            Operator::I64LtS => ctx.i64_lt_s(),
            Operator::I64LeS => ctx.i64_le_s(),
            Operator::I64GtS => ctx.i64_gt_s(),
            Operator::I64GeS => ctx.i64_ge_s(),
            Operator::I64LtU => ctx.i64_lt_u(),
            Operator::I64LeU => ctx.i64_le_u(),
            Operator::I64GtU => ctx.i64_gt_u(),
            Operator::I64GeU => ctx.i64_ge_u(),
            Operator::I64Add => ctx.i64_add(),
            Operator::I64Sub => ctx.i64_sub(),
            Operator::I64And => ctx.i64_and(),
            Operator::I64Or => ctx.i64_or(),
            Operator::I64Xor => ctx.i64_xor(),
            Operator::I64Mul => ctx.i64_mul(),
            Operator::I64Clz => ctx.i64_clz(),
            Operator::I64Ctz => ctx.i64_ctz(),
            Operator::I64Popcnt => ctx.i64_popcnt(),
            Operator::Drop => ctx.drop(),
            Operator::SetLocal { local_index } => ctx.set_local(local_index),
            Operator::GetLocal { local_index } => ctx.get_local(local_index),
            Operator::TeeLocal { local_index } => ctx.tee_local(local_index),
            Operator::I32Const { value } => ctx.i32_literal(value),
            Operator::I64Const { value } => ctx.i64_literal(value),
            Operator::I32Load { memarg } => ctx.i32_load(memarg.offset)?,
            Operator::I64Load { memarg } => ctx.i64_load(memarg.offset)?,
            Operator::I32Store { memarg } => ctx.i32_store(memarg.offset)?,
            Operator::I64Store { memarg } => ctx.i64_store(memarg.offset)?,
            Operator::Call { function_index } => {
                let callee_ty = translation_ctx.func_type(function_index);

                // TODO: this implementation assumes that this function is locally defined.

                ctx.call_direct(
                    function_index,
                    callee_ty.params.len() as u32,
                    callee_ty.returns.len() as u32,
                );
            }
            Operator::CallIndirect { index, table_index } => {
                assert_eq!(table_index, 0);

                let callee_ty = translation_ctx.signature(index);

                // TODO: this implementation assumes that this function is locally defined.

                ctx.call_indirect(
                    (0..translation_ctx.func_count() as u32)
                        .filter(|i| translation_ctx.func_type_index(*i) == index),
                    callee_ty.params.len() as u32,
                    callee_ty.returns.len() as u32,
                );
            }
            Operator::Nop => {}
            op => {
                unimplemented!("{:?}", op);
            }
        }
    }
    ctx.epilogue(func);

    Ok(())
}
