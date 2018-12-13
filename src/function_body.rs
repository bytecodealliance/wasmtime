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
    #[allow(unused)]
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
    fn br_destination(&self) -> Label {
        match *self {
            ControlFrameKind::Block { end_label } => end_label,
            ControlFrameKind::Loop { header } => header,
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
    /// Relative stack depth at the beginning of the frame.
    stack_depth: StackDepth,
    ty: Type,
}

impl ControlFrame {
    pub fn new(kind: ControlFrameKind, stack_depth: StackDepth, ty: Type) -> ControlFrame {
        ControlFrame {
            kind,
            stack_depth,
            ty,
            stack_polymorphic: false,
        }
    }

    pub fn outgoing_stack_depth(&self) -> StackDepth {
        let mut outgoing_stack_depth = self.stack_depth;
        if self.ty != Type::EmptyBlockType {
            // If there a return value then reserve expected outgoing stack depth value
            // to account for the result value.
            outgoing_stack_depth.reserve(1);
        }
        outgoing_stack_depth
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

    let mut framesize = arg_count;
    for local in locals {
        let (count, _ty) = local?;
        framesize += count;
    }

    let mut ctx = session.new_context(func_idx);
    let operators = body.get_operators_reader()?;

    prologue(&mut ctx, framesize);

    for arg_pos in 0..arg_count {
        copy_incoming_arg(&mut ctx, arg_pos);
    }

    let mut control_frames = Vec::new();

    // Upon entering the function implicit frame for function body is pushed. It has the same
    // result type as the function itself. Branching to it is equivalent to returning from the function.
    let epilogue_label = create_label(&mut ctx);
    control_frames.push(ControlFrame::new(
        ControlFrameKind::Block {
            end_label: epilogue_label,
        },
        current_stack_depth(&ctx),
        return_ty,
    ));

    for op in operators {
        match op? {
            Operator::Unreachable => {
                control_frames
                    .last_mut()
                    .expect("control stack is never empty")
                    .mark_stack_polymorphic();
                trap(&mut ctx);
            }
            Operator::If { ty } => {
                let end_label = create_label(&mut ctx);
                let if_not = create_label(&mut ctx);

                pop_and_breq(&mut ctx, if_not);

                control_frames.push(ControlFrame::new(
                    ControlFrameKind::IfTrue { end_label, if_not },
                    current_stack_depth(&ctx),
                    ty,
                ));
            }
            Operator::Else => {
                match control_frames.pop() {
                    Some(ControlFrame {
                        kind: ControlFrameKind::IfTrue { if_not, end_label },
                        ty,
                        stack_depth,
                        ..
                    }) => {
                        // Finalize if..else block by jumping to the `end_label`.
                        br(&mut ctx, end_label);

                        // Define `if_not` label here, so if the corresponding `if` block receives
                        // 0 it will branch here.
                        // After that reset stack depth to the value before entering `if` block.
                        define_label(&mut ctx, if_not);
                        restore_stack_depth(&mut ctx, stack_depth);

                        // Carry over the `end_label`, so it will be resolved when the corresponding `end`
                        // is encountered.
                        //
                        // Also note that we reset `stack_depth` to the value before entering `if` block.
                        let mut frame = ControlFrame::new(
                            ControlFrameKind::IfFalse { end_label },
                            stack_depth,
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

                if !control_frame.kind.is_loop() {
                    // Branches to a control frame with block type directs control flow to the header of the loop
                    // and we don't need to resolve it here. Branching to other control frames always lead
                    // control flow to the corresponding `end`.
                    define_label(&mut ctx, control_frame.kind.br_destination());
                }

                if let ControlFrameKind::IfTrue { if_not, .. } = control_frame.kind {
                    // this is `if .. end` construction. Define the `if_not` label here.
                    define_label(&mut ctx, if_not);
                }

                restore_stack_depth(&mut ctx, control_frame.outgoing_stack_depth());

                if control_frames.len() == 0 {
                    // This is the last control frame. Perform the implicit return here.
                    if return_ty != Type::EmptyBlockType {
                        prepare_return_value(&mut ctx);
                    }
                }
            }
            Operator::I32Eq => {
                relop_eq_i32(&mut ctx);
            }
            Operator::I32Add => {
                add_i32(&mut ctx);
            }
            Operator::GetLocal { local_index } => {
                get_local_i32(&mut ctx, local_index);
            }
            Operator::I32Const { value } => {
                literal_i32(&mut ctx, value);
            }
            Operator::Call { function_index } => {
                let callee_ty = translation_ctx.func_type(function_index);

                // TODO: this implementation assumes that this function is locally defined.
                // TODO: guarantee 16-byte alignment for calls as required by x86-64 ABI

                pass_outgoing_args(&mut ctx, callee_ty.params.len() as u32);
                call_direct(&mut ctx, function_index, callee_ty.returns.len() as u32);
            }
            _ => {
                trap(&mut ctx);
            }
        }
    }
    epilogue(&mut ctx);

    Ok(())
}
