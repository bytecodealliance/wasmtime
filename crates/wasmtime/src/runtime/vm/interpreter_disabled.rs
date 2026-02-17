//! Stubs for when pulley is disabled at compile time.
//!
//! Note that this is structured so that these structures are all zero-sized and
//! `Option<Thing>` is also zero-sized so there should be no runtime cost for
//! having these structures plumbed around.

use crate::runtime::vm::{VMContext, VMOpaqueContext};
use crate::{Engine, ValRaw, error::OutOfMemory};
use core::convert::Infallible;
use core::marker;
use core::mem;
use core::ptr::NonNull;
use wasmtime_unwinder::Unwind;

pub struct Interpreter {
    empty: Infallible,
}

const _: () = assert!(mem::size_of::<Interpreter>() == 0);
const _: () = assert!(mem::size_of::<Option<Interpreter>>() == 0);

impl Interpreter {
    pub fn new(_engine: &Engine) -> Result<Interpreter, OutOfMemory> {
        unreachable!()
    }

    pub fn as_interpreter_ref(&mut self) -> InterpreterRef<'_> {
        match self.empty {}
    }

    pub fn unwinder(&self) -> &'static dyn Unwind {
        match self.empty {}
    }
}

pub struct InterpreterRef<'a> {
    empty: Infallible,
    _marker: marker::PhantomData<&'a mut Interpreter>,
}

const _: () = assert!(mem::size_of::<InterpreterRef<'_>>() == 0);
const _: () = assert!(mem::size_of::<Option<InterpreterRef<'_>>>() == 0);

impl InterpreterRef<'_> {
    pub unsafe fn call(
        self,
        _bytecode: NonNull<u8>,
        _callee: NonNull<VMOpaqueContext>,
        _caller: NonNull<VMContext>,
        _args_and_results: NonNull<[ValRaw]>,
    ) -> bool {
        match self.empty {}
    }

    pub(crate) unsafe fn resume_to_exception_handler(
        &mut self,
        _handler: &wasmtime_unwinder::Handler,
        _payload1: usize,
        _payload2: usize,
    ) {
        match self.empty {}
    }
}
