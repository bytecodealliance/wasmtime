use anyhow::Result;
use wasmtime::*;

#[test]
fn use_func_after_drop() -> Result<()> {
    let table;
    {
        let store = Store::default();
        let closed_over_data = String::from("abcd");
        let func = Func::wrap(&store, move || {
            assert_eq!(closed_over_data, "abcd");
        });
        let ty = TableType::new(ValType::FuncRef, Limits::new(1, None));
        table = Table::new(&store, ty, Val::ExternRef(None))?;
        table.set(0, func.into())?;
    }
    let func = table.get(0).unwrap().funcref().unwrap().clone();
    let func = func.get0::<()>()?;
    func()?;
    Ok(())
}
