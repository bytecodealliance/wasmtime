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
    Loop { header: Label, break_: Label },
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
    fn block_end(&self) -> Label {
        match *self {
            ControlFrameKind::Block { end_label } => end_label,
            ControlFrameKind::Loop { break_, .. } => break_,
            ControlFrameKind::IfTrue { end_label, .. } => end_label,
            ControlFrameKind::IfFalse { end_label } => end_label,
        }
    }

    /// Returns `true` if this block of a loop kind.
    fn is_loop(&self) -> bool {
        match *self {
            ControlFrameKind::Loop { .. } => true,
            _ => false,
        }
    }
}

struct ControlFrame {
    kind: ControlFrameKind,
    /// Boolean which signals whether value stack became polymorphic. Value stack starts in non-polymorphic state and
    /// becomes polymorphic only after an instruction that never passes control further is executed,
    /// i.e. `unreachable`, `br` (but not `br_if`!), etc.
    stack_polymorphic: bool,
    /// State specific to the block (free temp registers, stack etc) which should be replaced
    /// at the end of the block
    block_state: BlockState,
    ty: Type,
}

impl ControlFrame {
    pub fn new(kind: ControlFrameKind, block_state: BlockState, ty: Type) -> ControlFrame {
        ControlFrame {
            kind,
            block_state,
            ty,
            stack_polymorphic: false,
        }
    }

    /// Marks this control frame as reached stack-polymorphic state.
    pub fn mark_stack_polymorphic(&mut self) {
        self.stack_polymorphic = true;
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
    let return_ty = if func_type.returns.len() > 0 {
        func_type.returns[0]
    } else {
        Type::EmptyBlockType
    };

    let mut num_locals = 0;
    for local in locals {
        let (count, _ty) = local?;
        num_locals += count;
    }

    let ctx = &mut session.new_context(func_idx);
    let operators = body.get_operators_reader()?;

    start_function(ctx, arg_count, num_locals);

    let mut control_frames = Vec::new();

    // Upon entering the function implicit frame for function body is pushed. It has the same
    // result type as the function itself. Branching to it is equivalent to returning from the function.
    let epilogue_label = create_label(ctx);
    control_frames.push(ControlFrame::new(
        ControlFrameKind::Block {
            end_label: epilogue_label,
        },
        current_block_state(&ctx),
        return_ty,
    ));

    for op in operators {
        match op? {
            Operator::Unreachable => {
                control_frames
                    .last_mut()
                    .expect("control stack is never empty")
                    .mark_stack_polymorphic();
                trap(ctx);
            }
            Operator::If { ty } => {
                let end_label = create_label(ctx);
                let if_not = create_label(ctx);

                pop_and_breq(ctx, if_not);

                control_frames.push(ControlFrame::new(
                    ControlFrameKind::IfTrue { end_label, if_not },
                    current_block_state(&ctx),
                    ty,
                ));
            }
            Operator::Loop { ty } => {
                let header = create_label(ctx);
                let break_ = create_label(ctx);

                define_label(ctx, header);
                pop_and_breq(ctx, break_);

                control_frames.push(ControlFrame::new(
                    ControlFrameKind::Loop { header, break_ },
                    current_block_state(&ctx),
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
                        if ty != Type::EmptyBlockType {
                            return_from_block(ctx);
                        }

                        // Finalize if..else block by jumping to the `end_label`.
                        br(ctx, end_label);

                        // Define `if_not` label here, so if the corresponding `if` block receives
                        // 0 it will branch here.
                        // After that reset stack depth to the value before entering `if` block.
                        define_label(ctx, if_not);
                        end_block(ctx, block_state.clone());

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

                if control_frame.ty != Type::EmptyBlockType && !control_frames.is_empty() {
                    return_from_block(ctx);
                }

                if let ControlFrameKind::Loop { header, .. } = control_frame.kind {
                    br(ctx, header);
                }

                define_label(ctx, control_frame.kind.block_end());

                if let ControlFrameKind::IfTrue { if_not, .. } = control_frame.kind {
                    // this is `if .. end` construction. Define the `if_not` label here.
                    define_label(ctx, if_not);
                }

                // This is the last control frame. Perform the implicit return here.
                if control_frames.len() == 0 && return_ty != Type::EmptyBlockType {
                    prepare_return_value(ctx);
                }

                end_block(ctx, control_frame.block_state);
                push_block_return_value(ctx);
            }
            Operator::I32Eq => relop_eq_i32(ctx),
            Operator::I32Add => i32_add(ctx),
            Operator::I32Sub => i32_sub(ctx),
            Operator::I32And => i32_and(ctx),
            Operator::I32Or => i32_or(ctx),
            Operator::I32Xor => i32_xor(ctx),
            Operator::I32Mul => i32_mul(ctx),
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
                push_return_value(ctx);
            }
            _ => {
                trap(ctx);
            }
        }
    }
    epilogue(ctx);

    Ok(())
}
