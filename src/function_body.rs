use backend::*;
use error::Error;
use module::TranslationContext;
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
    translation_ctx: &TranslationContext,
    func_idx: u32,
    body: &FunctionBody,
) -> Result<(), Error> {
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

    let func = start_function(ctx, arg_count, num_locals);

    let mut control_frames = Vec::new();

    // Upon entering the function implicit frame for function body is pushed. It has the same
    // result type as the function itself. Branching to it is equivalent to returning from the function.
    let epilogue_label = create_label(ctx);
    let function_block_state = start_block(ctx, arity(return_ty));
    control_frames.push(ControlFrame::new(
        ControlFrameKind::Block {
            end_label: epilogue_label,
        },
        function_block_state,
        return_ty,
    ));

    for op in operators {
        let op = op?;

        if let Operator::End = op {
        } else {
            if control_frames
                .last()
                .expect("Control stack never empty")
                .unreachable
            {
                continue;
            }
        }

        match op {
            Operator::Unreachable => {
                control_frames
                    .last_mut()
                    .expect("control stack is never empty")
                    .mark_unreachable();
                trap(ctx);
            }
            Operator::Block { ty } => {
                let label = create_label(ctx);
                let state = start_block(ctx, arity(ty));
                control_frames.push(ControlFrame::new(
                    ControlFrameKind::Block { end_label: label },
                    state,
                    ty,
                ));
            }
            Operator::Br { relative_depth } => {
                control_frames
                    .last_mut()
                    .expect("control stack is never empty")
                    .mark_unreachable();

                let idx = control_frames.len() - 1 - relative_depth as usize;
                let control_frame = control_frames.get(idx).expect("wrong depth");

                return_from_block(ctx, control_frame.arity(), idx == 0);

                br(ctx, control_frame.kind.branch_target());
            }
            Operator::BrIf { relative_depth } => {
                let idx = control_frames.len() - 1 - relative_depth as usize;
                let control_frame = control_frames.get(idx).expect("wrong depth");

                let if_not = create_label(ctx);

                jump_if_equal_zero(ctx, if_not);

                return_from_block(ctx, control_frame.arity(), idx == 0);
                br(ctx, control_frame.kind.branch_target());

                define_label(ctx, if_not);
            }
            Operator::If { ty } => {
                let end_label = create_label(ctx);
                let if_not = create_label(ctx);

                jump_if_equal_zero(ctx, if_not);
                let state = start_block(ctx, arity(ty));

                control_frames.push(ControlFrame::new(
                    ControlFrameKind::IfTrue { end_label, if_not },
                    state,
                    ty,
                ));
            }
            Operator::Loop { ty } => {
                let header = create_label(ctx);

                let state = start_block(ctx, arity(ty));
                define_label(ctx, header);

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
                        return_from_block(ctx, arity(ty), false);
                        end_block(ctx, block_state.clone(), arity(ty));

                        // Finalize `then` block by jumping to the `end_label`.
                        br(ctx, end_label);

                        // Define `if_not` label here, so if the corresponding `if` block receives
                        // 0 it will branch here.
                        // After that reset stack depth to the value before entering `if` block.
                        define_label(ctx, if_not);

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
                let control_frame = control_frames.pop().expect("control stack is never empty");

                let arity = control_frame.arity();

                // Don't bother generating this code if we're in unreachable code
                if !control_frame.unreachable {
                    return_from_block(ctx, arity, control_frames.is_empty());
                }

                end_block(ctx, control_frame.block_state, arity);

                if let Some(block_end) = control_frame.kind.block_end() {
                    define_label(ctx, block_end);
                }

                if let ControlFrameKind::IfTrue { if_not, .. } = control_frame.kind {
                    // this is `if .. end` construction. Define the `if_not` label here.
                    define_label(ctx, if_not);
                }
            }
            Operator::I32Eq => relop_eq_i32(ctx),
            Operator::I32Add => i32_add(ctx),
            Operator::I32Sub => i32_sub(ctx),
            Operator::I32And => i32_and(ctx),
            Operator::I32Or => i32_or(ctx),
            Operator::I32Xor => i32_xor(ctx),
            Operator::I32Mul => i32_mul(ctx),
            Operator::Drop => drop(ctx),
            Operator::SetLocal { local_index } => set_local_i32(ctx, local_index),
            Operator::GetLocal { local_index } => get_local_i32(ctx, local_index),
            Operator::I32Const { value } => literal_i32(ctx, value),
            Operator::Call { function_index } => {
                let callee_ty = translation_ctx.func_type(function_index);

                // TODO: this implementation assumes that this function is locally defined.

                call_direct(
                    ctx,
                    function_index,
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
    epilogue(ctx, func);

    Ok(())
}
