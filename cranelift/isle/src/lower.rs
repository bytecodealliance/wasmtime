use crate::ir::*;
use crate::sema;

struct LowerState<'a> {
    tyenv: &'a sema::TypeEnv,
    func: &'a sema::Func,
    builder: FuncBuilder,
    control_flow: ControlInput,
}

pub fn lower(tyenv: &sema::TypeEnv, func: &sema::Func) -> Func {
    let mut builder = FuncBuilder::default();
    let entry = builder.intern(Node::Entry);

    let mut state = LowerState {
        tyenv,
        func,
        builder,
        control_flow: ControlInput(entry, 0),
    };

    if !func.is_extern && !func.is_inline {
        for case in &func.cases {
            state.lower_case(case);
        }
    }

    state.builder.build()
}

impl<'a> LowerState<'a> {
    fn lower_case(&mut self) {}
}
