//! Stubs for when pulley is disabled at compile time.
//!
//! Note that this is structured so that these structures are all zero-sized and
//! `Option<Thing>` is also zero-sized so there should be no runtime cost for
//! having these structures plumbed around.

use crate::runtime::vm::VMOpaqueContext;
use crate::runtime::Uninhabited;
use crate::ValRaw;
use core::marker;
use core::mem;
use core::ptr::NonNull;

pub struct Interpreter {
    empty: Uninhabited,
}

const _: () = assert!(mem::size_of::<Interpreter>() == 0);
const _: () = assert!(mem::size_of::<Option<Interpreter>>() == 0);

impl Interpreter {
    pub fn new() -> Interpreter {
        unreachable!()
    }

    pub fn as_interpreter_ref(&mut self) -> InterpreterRef<'_> {
        match self.empty {}
    }
}

pub struct InterpreterRef<'a> {
    empty: Uninhabited,
    _marker: marker::PhantomData<&'a mut Interpreter>,
}

const _: () = assert!(mem::size_of::<InterpreterRef<'_>>() == 0);
const _: () = assert!(mem::size_of::<Option<InterpreterRef<'_>>>() == 0);

impl InterpreterRef<'_> {
    pub unsafe fn call(
        self,
        _bytecode: NonNull<u8>,
        _callee: *mut VMOpaqueContext,
        _caller: *mut VMOpaqueContext,
        _args_and_results: *mut [ValRaw],
    ) -> bool {
        match self.empty {}
    }
}
