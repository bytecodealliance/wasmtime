use backend::*;
use error::Error;
use wasmparser::{FunctionBody, Operator};

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
    // TODO: type, stack height, etc
}

impl ControlFrame {
    pub fn new(kind: ControlFrameKind) -> ControlFrame {
        ControlFrame {
            kind,
            stack_polymorphic: false,
        }
    }

    /// Marks this control frame as reached stack-polymorphic state.
    pub fn mark_stack_polymorphic(&mut self) {
        self.stack_polymorphic = true;
    }
}

pub fn translate(session: &mut CodeGenSession, body: &FunctionBody) -> Result<(), Error> {
    let locals = body.get_locals_reader()?;

    // Assume signature is (i32, i32) -> i32 for now.
    // TODO: Use a real signature
    const ARG_COUNT: u32 = 2;

    let mut framesize = ARG_COUNT;
    for local in locals {
        let (count, _ty) = local?;
        framesize += count;
    }

    let mut ctx = session.new_context();
    let operators = body.get_operators_reader()?;

    prologue(&mut ctx, framesize);

    for arg_pos in 0..ARG_COUNT {
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
                control_frames.push(ControlFrame::new(
                    ControlFrameKind::IfTrue {
                        end_label,
                        if_not,
                    },
                ));

                // TODO: Generate code that pops a value and executes the if_true part if the value
                // is not equal to zero.
            }
            Operator::End => {
                let control_frame = control_frames.pop().expect("control stack is never empty");
                if !control_frame.kind.is_loop() {
                    // Branches to a control frame with block type directs control flow to the header of the loop 
                    // and we don't need to resolve it here. Branching to other control frames always lead
                    // control flow to the corresponding `end`.
                    define_label(&mut ctx, control_frame.kind.br_destination());
                }
            }
            Operator::I32Add => {
                add_i32(&mut ctx);
            }
            Operator::GetLocal { local_index } => {
                get_local_i32(&mut ctx, local_index);
            }
            _ => {
                trap(&mut ctx);
            }
        }
    }
    prepare_return_value(&mut ctx);
    epilogue(&mut ctx);

    Ok(())
}
