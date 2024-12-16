use crate::prelude::*;
use crate::runtime::vm::vmcontext::VMArrayCallNative;
use crate::runtime::vm::{tls, TrapRegisters, TrapTest, VMContext, VMOpaqueContext};
use crate::{Engine, ValRaw};
use core::ptr::NonNull;
use pulley_interpreter::interp::{DoneReason, RegType, TrapKind, Val, Vm, XRegVal};
use pulley_interpreter::{Reg, XReg};
use wasmtime_environ::{BuiltinFunctionIndex, HostCall, Trap};

/// Interpreter state stored within a `Store<T>`.
#[repr(transparent)]
pub struct Interpreter {
    /// Pulley VM state, stored behind a `Box<T>` to make the storage in
    /// `Store<T>` only pointer-sized (that way if you enable pulley but don't
    /// use it it's low-overhead).
    pulley: Box<Vm>,
}

impl Interpreter {
    /// Creates a new interpreter ready to interpret code.
    pub fn new(engine: &Engine) -> Interpreter {
        Interpreter {
            pulley: Box::new(Vm::with_stack(vec![0; engine.config().max_wasm_stack])),
        }
    }

    /// Returns the `InterpreterRef` structure which can be used to actually
    /// execute interpreted code.
    pub fn as_interpreter_ref(&mut self) -> InterpreterRef<'_> {
        InterpreterRef(&mut self.pulley)
    }
}

/// Wrapper around `&mut pulley_interpreter::Vm` to enable compiling this to a
/// zero-sized structure when pulley is disabled at compile time.
#[repr(transparent)]
pub struct InterpreterRef<'a>(&'a mut Vm);

#[derive(Clone, Copy)]
struct Setjmp {
    sp: *mut u8,
    fp: *mut u8,
    lr: *mut u8,
}

impl InterpreterRef<'_> {
    /// Invokes interpreted code.
    ///
    /// The `bytecode` pointer should previously have been produced by Cranelift
    /// and `callee` / `caller` / `args_and_results` are normal array-call
    /// arguments being passed around.
    pub unsafe fn call(
        mut self,
        mut bytecode: NonNull<u8>,
        callee: *mut VMOpaqueContext,
        caller: *mut VMOpaqueContext,
        args_and_results: *mut [ValRaw],
    ) -> bool {
        // Initialize argument registers with the ABI arguments.
        let args = [
            XRegVal::new_ptr(callee).into(),
            XRegVal::new_ptr(caller).into(),
            XRegVal::new_ptr(args_and_results.cast::<u8>()).into(),
            XRegVal::new_u64(args_and_results.len() as u64).into(),
        ];

        // Fake a "poor man's setjmp" for now by saving some critical context to
        // get restored when a trap happens. This pseudo-implements the stack
        // unwinding necessary for a trap.
        //
        // See more comments in `trap` below about how this isn't actually
        // correct as it's not saving all callee-save state.
        let setjmp = Setjmp {
            sp: self.0[XReg::sp].get_ptr(),
            fp: self.0.fp(),
            lr: self.0.lr(),
        };

        let old_lr = self.0.call_start(&args);

        // Run the interpreter as much as possible until it finishes, and then
        // handle each finish condition differently.
        let ret = loop {
            match self.0.call_run(bytecode) {
                // If the VM returned entirely then read the return value and
                // return that (it indicates whether a trap happened or not.
                DoneReason::ReturnToHost(()) => {
                    match self.0.call_end(old_lr, [RegType::XReg]).next().unwrap() {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "intentionally reading the lower bits only"
                        )]
                        Val::XReg(xreg) => break (xreg.get_u32() as u8) != 0,
                        _ => unreachable!(),
                    }
                }
                // If the VM wants to call out to the host then dispatch that
                // here based on `sig`. Once that returns we can resume
                // execution at `resume`.
                //
                // Note that the `raise` libcall is handled specially here since
                // longjmp/setjmp is handled differently than on the host.
                DoneReason::CallIndirectHost { id, resume } => {
                    if u32::from(id) == HostCall::Builtin(BuiltinFunctionIndex::raise()).index() {
                        self.longjmp(setjmp);
                        break false;
                    } else {
                        self.call_indirect_host(id);
                        bytecode = resume;
                    }
                }
                // If the VM trapped then process that here and return `false`.
                DoneReason::Trap { pc, kind } => {
                    self.trap(pc, kind, setjmp);
                    break false;
                }
            }
        };

        debug_assert!(self.0[XReg::sp].get_ptr() == setjmp.sp);
        debug_assert!(self.0.fp() == setjmp.fp);
        debug_assert!(self.0.lr() == setjmp.lr);
        ret
    }

    /// Handles an interpreter trap. This will initialize the trap state stored
    /// in TLS via the `test_if_trap` helper below by reading the pc/fp of the
    /// interpreter and seeing if that's a valid opcode to trap at.
    fn trap(&mut self, pc: NonNull<u8>, kind: Option<TrapKind>, setjmp: Setjmp) {
        let regs = TrapRegisters {
            pc: pc.as_ptr() as usize,
            fp: self.0.fp() as usize,
        };
        tls::with(|s| {
            let s = s.unwrap();
            match kind {
                Some(kind) => {
                    let trap = match kind {
                        TrapKind::IntegerOverflow => Trap::IntegerOverflow,
                        TrapKind::DivideByZero => Trap::IntegerDivisionByZero,
                        TrapKind::BadConversionToInteger => Trap::BadConversionToInteger,
                    };
                    s.set_jit_trap(regs, None, trap);
                }
                None => {
                    match s.test_if_trap(regs, None, |_| false) {
                        // This shouldn't be possible, so this is a fatal error
                        // if it happens.
                        TrapTest::NotWasm => {
                            panic!("pulley trap at {pc:?} without trap code registered")
                        }

                        // Not possible with our closure above returning `false`.
                        TrapTest::HandledByEmbedder => unreachable!(),

                        // Trap was handled, yay! We don't use `jmp_buf`.
                        TrapTest::Trap { jmp_buf: _ } => {}
                    }
                }
            }
        });

        self.longjmp(setjmp);
    }

    /// Perform a "longjmp" by restoring the "setjmp" context saved when this
    /// started.
    ///
    /// FIXME: this is not restoring callee-save state. For example if
    /// there's more than one Pulley activation on the stack that means that
    /// the previous one is expecting the callee (the host) to preserve all
    /// callee-save registers. That's not restored here which means with
    /// multiple activations we're effectively corrupting callee-save
    /// registers.
    ///
    /// One fix for this is to possibly update the `SystemV` ABI on pulley to
    /// have no callee-saved registers and make everything caller-saved. That
    /// would force all trampolines to save all state which is basically
    /// what we want as they'll naturally restore state if we later return to
    /// them.
    fn longjmp(&mut self, setjmp: Setjmp) {
        let Setjmp { sp, fp, lr } = setjmp;
        unsafe {
            self.0[XReg::sp].set_ptr(sp);
            self.0.set_fp(fp);
            self.0.set_lr(lr);
        }
    }

    /// Handles the `call_indirect_host` instruction, dispatching the `sig`
    /// number here which corresponds to `wasmtime_environ::HostCall`.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "macro-generated code"
    )]
    #[cfg_attr(
        not(feature = "component-model"),
        expect(unused_macro_rules, reason = "macro-code")
    )]
    unsafe fn call_indirect_host(&mut self, id: u8) {
        let id = u32::from(id);
        let fnptr = self.0[XReg::x0].get_ptr::<u8>();
        let mut arg_reg = 1;

        /// Helper macro to invoke a builtin.
        ///
        /// Used as:
        ///
        /// `call(@builtin(ty1, ty2, ...) -> retty)` - invoke a core or
        /// component builtin with the macro-defined signature.
        ///
        /// `call(@host Ty(ty1, ty2, ...) -> retty)` - invoke a host function
        /// with the type `Ty`. The other types in the macro are checked by
        /// rustc to match the actual `Ty` definition in Rust.
        macro_rules! call {
            (@builtin($($param:ident),*) $(-> $result:ident)?) => {{
                type T = unsafe extern "C" fn($(call!(@ty $param)),*) $(-> call!(@ty $result))?;
                call!(@host T($($param),*) $(-> $result)?);
            }};
            (@host $ty:ident($($param:ident),*) $(-> $result:ident)?) => {{
                // Convert the pointer from pulley to a native function pointer.
                union GetNative {
                    fnptr: *mut u8,
                    host: $ty,
                }
                let host = GetNative { fnptr }.host;

                // Decode each argument according to this macro, pulling
                // arguments from successive registers.
                let ret = host($({
                    let reg = XReg::new(arg_reg).unwrap();
                    arg_reg += 1;
                    call!(@get $param reg)
                }),*);
                let _ = arg_reg; // silence last dead arg_reg increment warning

                // Store the return value, if one is here, in x0.
                $(
                    let dst = XReg::x0;
                    call!(@set $result dst ret);
                )?
                let _ = ret; // silence warning if no return value

                // Return from the outer `call_indirect_host` host function as
                // it's been processed.
                return;
            }};

            // Conversion from macro-defined types to Rust host types.
            (@ty bool) => (bool);
            (@ty u8) => (u8);
            (@ty u32) => (u32);
            (@ty i32) => (i32);
            (@ty u64) => (u64);
            (@ty i64) => (i64);
            (@ty vmctx) => (*mut VMContext);
            (@ty pointer) => (*mut u8);
            (@ty ptr_u8) => (*mut u8);
            (@ty ptr_u16) => (*mut u16);
            (@ty ptr_size) => (*mut usize);
            (@ty size) => (usize);

            // Conversion from a pulley register value to the macro-defined
            // type.
            (@get u8 $reg:ident) => (self.0[$reg].get_i32() as u8);
            (@get u32 $reg:ident) => (self.0[$reg].get_u32());
            (@get i32 $reg:ident) => (self.0[$reg].get_i32());
            (@get i64 $reg:ident) => (self.0[$reg].get_i64());
            (@get vmctx $reg:ident) => (self.0[$reg].get_ptr());
            (@get pointer $reg:ident) => (self.0[$reg].get_ptr());
            (@get ptr $reg:ident) => (self.0[$reg].get_ptr());
            (@get ptr_u8 $reg:ident) => (self.0[$reg].get_ptr());
            (@get ptr_u16 $reg:ident) => (self.0[$reg].get_ptr());
            (@get ptr_size $reg:ident) => (self.0[$reg].get_ptr());
            (@get size $reg:ident) => (self.0[$reg].get_ptr::<u8>() as usize);

            // Conversion from a Rust value back into a macro-defined type,
            // stored in a pulley register.
            (@set bool $reg:ident $val:ident) => (self.0[$reg].set_i32(i32::from($val)));
            (@set i32 $reg:ident $val:ident) => (self.0[$reg].set_i32($val));
            (@set u64 $reg:ident $val:ident) => (self.0[$reg].set_u64($val));
            (@set i64 $reg:ident $val:ident) => (self.0[$reg].set_i64($val));
            (@set pointer $reg:ident $val:ident) => (self.0[$reg].set_ptr($val));
            (@set size $reg:ident $val:ident) => (self.0[$reg].set_ptr($val as *mut u8));
        }

        // With the helper macro above structure this into:
        //
        // foreach [core, component]
        //   * dispatch the call-the-host function pointer type
        //   * dispatch all builtins by their index.
        //
        // The hope is that this is relatively easy for LLVM to optimize since
        // it's a bunch of:
        //
        //  if id == 0 { ...;  return; }
        //  if id == 1 { ...;  return; }
        //  if id == 2 { ...;  return; }
        //  ...
        //

        if id == const { HostCall::ArrayCall.index() } {
            call!(@host VMArrayCallNative(ptr, ptr, ptr, size) -> bool);
        }

        macro_rules! core {
            (
                $(
                    $( #[cfg($attr:meta)] )?
                    $name:ident($($pname:ident: $param:ident ),* ) $(-> $result:ident)?;
                )*
            ) => {
                $(
                    $( #[cfg($attr)] )?
                    if id == const { HostCall::Builtin(BuiltinFunctionIndex::$name()).index() } {
                        call!(@builtin($($param),*) $(-> $result)?);
                    }
                )*
            }
        }
        wasmtime_environ::foreach_builtin_function!(core);

        #[cfg(feature = "component-model")]
        {
            use crate::runtime::vm::component::VMLoweringCallee;
            use wasmtime_environ::component::ComponentBuiltinFunctionIndex;

            if id == const { HostCall::ComponentLowerImport.index() } {
                call!(@host VMLoweringCallee(ptr, ptr, u32, ptr, ptr, ptr, u8, ptr, size) -> bool);
            }

            macro_rules! component {
                (
                    $(
                        $name:ident($($pname:ident: $param:ident ),* ) $(-> $result:ident)?;
                    )*
                ) => {
                    $(
                        if id == const { HostCall::ComponentBuiltin(ComponentBuiltinFunctionIndex::$name()).index() } {
                            call!(@builtin($($param),*) $(-> $result)?);
                        }
                    )*
                }
            }
            wasmtime_environ::foreach_builtin_component_function!(component);
        }

        // if we got this far then something has gone seriously wrong.
        unreachable!()
    }
}
