use anyhow::Result;
use wasmtime::*;

enum State {
    Native,
    Vm,
}

impl Default for State {
    fn default() -> Self {
        State::Native
    }
}

#[test]
fn call_wrapped_func() -> Result<()> {
    let mut store = Store::<State>::default();
    store.entering_native_code_hook(|s| match &s {
        State::Vm => {
            println!("entering native");
            *s = State::Native;
            Ok(())
        }
        State::Native => Err(Trap::new("illegal state: exiting vm when in native")),
    });
    store.exiting_native_code_hook(|s| match &s {
        State::Native => {
            println!("entering vm");
            *s = State::Vm;
            Ok(())
        }
        State::Vm => Err(Trap::new("illegal state: exiting native when in vm")),
    });
    let f = Func::wrap(&mut store, |a: i32, b: i64, c: f32, d: f64| {
        assert_eq!(a, 1);
        assert_eq!(b, 2);
        assert_eq!(c, 3.0);
        assert_eq!(d, 4.0);
    });

    println!("untyped call");
    f.call(
        &mut store,
        &[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()],
    )?;

    println!("typed call");
    f.typed::<(i32, i64, f32, f64), (), _>(&store)?
        .call(&mut store, (1, 2, 3.0, 4.0))?;

    Ok(())
}
