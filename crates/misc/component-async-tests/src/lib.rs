#![expect(clippy::allow_attributes_without_reason)]

use std::sync::{Arc, Mutex};
use std::task::Waker;

use wasmtime::component::{HasData, ResourceTable};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiView};

pub mod borrowing_host;
pub mod closed_streams;
pub mod resource_stream;
pub mod round_trip;
pub mod round_trip_direct;
pub mod round_trip_many;
pub mod sleep;
pub mod transmit;
pub mod util;
pub mod yield_host;

/// Host implementation, usable primarily by tests
pub struct Ctx {
    pub wasi: WasiCtx,
    pub table: ResourceTable,
    pub wakers: Arc<Mutex<Option<Vec<Waker>>>>,
    pub continue_: bool,
}

impl IoView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl HasData for Ctx {
    type Data<'a> = &'a mut Self;
}
