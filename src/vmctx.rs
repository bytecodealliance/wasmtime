//! This code borrows heavily from Lucet's Vmctx implementation
//! https://github.com/fastly/lucet/blob/master/lucet-runtime/lucet-runtime-internals/src/vmctx.rs

use crate::ctx::WasiCtx;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Ref, RefCell, RefMut};

pub trait AsVmContextView {
    unsafe fn as_vm_context_view(&mut self) -> VmContextView;
}

#[derive(Debug)]
pub struct VmContextView {
    pub memory_view: RefCell<Box<[u8]>>,
    pub wasi_ctx_view: RefCell<Box<WasiCtx>>,
}

impl Drop for VmContextView {
    fn drop(&mut self) {
        let memory_view = self.memory_view.replace(Box::new([]));
        let wasi_ctx_view = self.wasi_ctx_view.replace(Box::new(WasiCtx::default()));
        Box::leak(memory_view);
        Box::leak(wasi_ctx_view);
    }
}

impl VmContextView {
    pub fn memory(&self) -> Ref<[u8]> {
        let r = self
            .memory_view
            .try_borrow()
            .expect("memory not already borrowed mutably");
        Ref::map(r, |b| b.borrow())
    }

    pub fn memory_mut(&self) -> RefMut<[u8]> {
        let r = self
            .memory_view
            .try_borrow_mut()
            .expect("memory not already borrowed");
        RefMut::map(r, |b| b.borrow_mut())
    }

    pub fn get_wasi_ctx(&self) -> Ref<WasiCtx> {
        let r = self
            .wasi_ctx_view
            .try_borrow()
            .expect("WASI context not already borrowed mutably");
        Ref::map(r, |b| b.borrow())
    }

    pub fn get_wasi_ctx_mut(&self) -> RefMut<WasiCtx> {
        let r = self
            .wasi_ctx_view
            .try_borrow_mut()
            .expect("WASI context not already borrowed");
        RefMut::map(r, |b| b.borrow_mut())
    }
}
