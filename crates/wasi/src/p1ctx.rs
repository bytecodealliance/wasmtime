use crate::preview1::{WasiPreview1Adapter, WasiPreview1View};
use crate::{WasiCtx, WasiView};
use wasmtime::component::ResourceTable;

pub struct WasiP1Ctx {
    pub table: ResourceTable,
    pub wasi: WasiCtx,
    pub adapter: WasiPreview1Adapter,
}

impl WasiP1Ctx {
    pub fn new(wasi: WasiCtx) -> Self {
        Self {
            table: ResourceTable::new(),
            wasi,
            adapter: WasiPreview1Adapter::new(),
        }
    }
}

impl WasiView for WasiP1Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl WasiPreview1View for WasiP1Ctx {
    fn adapter(&self) -> &WasiPreview1Adapter {
        &self.adapter
    }
    fn adapter_mut(&mut self) -> &mut WasiPreview1Adapter {
        &mut self.adapter
    }
}
