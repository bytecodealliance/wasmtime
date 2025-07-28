use anyhow::Result;
use wasmtime::component::Resource;
use wasmtime_wasi::p2::IoView;

use super::Ctx;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "borrowing-host",
        imports: { default: trappable },
        with: {
            "local:local/borrowing-types/x": super::MyX,
        }
    });
}

/// Used as the borrowing type (`local:local/borrowing-types/x`)
pub struct MyX;

impl bindings::local::local::borrowing_types::HostX for &mut Ctx {
    fn new(&mut self) -> Result<Resource<MyX>> {
        Ok(IoView::table(self).push(MyX)?)
    }

    fn foo(&mut self, x: Resource<MyX>) -> Result<()> {
        _ = IoView::table(self).get(&x)?;
        Ok(())
    }

    fn drop(&mut self, x: Resource<MyX>) -> Result<()> {
        IoView::table(self).delete(x)?;
        Ok(())
    }
}

impl bindings::local::local::borrowing_types::Host for &mut Ctx {}
