//! Cretonne IL builder library.
//!
//! Provides a straightforward way to create a Cretonne IL function and fill it with instructions
//! translated from another language. Contains a SSA construction module that lets you translate
//! your non-SSA variables into SSA Cretonne IL values via `use_var` and `def_var` calls.
//!
//! To get started, create an [`IlBuilder`](struct.ILBuilder.html) and pass it as an argument
//! to a [`FunctionBuilder`](struct.FunctionBuilder.html).
//!
//! # Example
//!
//! Here is a pseudo-program we want to transform into Cretonne IL:
//!
//! ```cton
//! function(x) {
//! x, y, z : i32
//! block0:
//!    y = 2;
//!    z = x + y;
//!    jump block1
//! block1:
//!    z = z + y;
//!    brnz y, block2;
//!    z = z - x;
//!    return y
//! block2:
//!    y = y - x
//!    jump block1
//! }
//! ```
//!
//! Here is how you build the corresponding Cretonne IL function using `ILBuilder`:
//!
//! ```rust
//! extern crate cretonne;
//! extern crate cton_frontend;
//!
//! use cretonne::entity::EntityRef;
//! use cretonne::ir::{ExternalName, CallConv, Function, Signature, AbiParam, InstBuilder};
//! use cretonne::ir::types::*;
//! use cretonne::settings;
//! use cton_frontend::{ILBuilder, FunctionBuilder};
//! use cretonne::verifier::verify_function;
//! use std::u32;
//!
//! // An opaque reference to variable.
//! #[derive(Copy, Clone, PartialEq, Eq, Debug)]
//! pub struct Variable(u32);
//! impl EntityRef for Variable {
//!     fn new(index: usize) -> Self {
//!         assert!(index < (u32::MAX as usize));
//!         Variable(index as u32)
//!     }
//!
//!     fn index(self) -> usize {
//!         self.0 as usize
//!     }
//! }
//!
//! fn main() {
//!     let mut sig = Signature::new(CallConv::Native);
//!     sig.returns.push(AbiParam::new(I32));
//!     sig.params.push(AbiParam::new(I32));
//!     let mut il_builder = ILBuilder::<Variable>::new();
//!     let mut func = Function::with_name_signature(ExternalName::new("sample_function"), sig);
//!     {
//!         let mut builder = FunctionBuilder::<Variable>::new(&mut func, &mut il_builder);
//!
//!         let block0 = builder.create_ebb();
//!         let block1 = builder.create_ebb();
//!         let block2 = builder.create_ebb();
//!         let x = Variable(0);
//!         let y = Variable(1);
//!         let z = Variable(2);
//!         builder.declare_var(x, I32);
//!         builder.declare_var(y, I32);
//!         builder.declare_var(z, I32);
//!
//!         builder.switch_to_block(block0);
//!         builder.seal_block(block0);
//!         {
//!             let tmp = builder.param_value(0);
//!             builder.def_var(x, tmp);
//!         }
//!         {
//!             let tmp = builder.ins().iconst(I32, 2);
//!             builder.def_var(y, tmp);
//!         }
//!         {
//!             let arg1 = builder.use_var(x);
//!             let arg2 = builder.use_var(y);
//!             let tmp = builder.ins().iadd(arg1, arg2);
//!             builder.def_var(z, tmp);
//!         }
//!         builder.ins().jump(block1, &[]);
//!
//!         builder.switch_to_block(block1);
//!         {
//!             let arg1 = builder.use_var(y);
//!             let arg2 = builder.use_var(z);
//!             let tmp = builder.ins().iadd(arg1, arg2);
//!             builder.def_var(z, tmp);
//!         }
//!         {
//!             let arg = builder.use_var(y);
//!             builder.ins().brnz(arg, block2, &[]);
//!         }
//!         {
//!             let arg1 = builder.use_var(z);
//!             let arg2 = builder.use_var(x);
//!             let tmp = builder.ins().isub(arg1, arg2);
//!             builder.def_var(z, tmp);
//!         }
//!         {
//!             let arg = builder.use_var(y);
//!             builder.ins().return_(&[arg]);
//!         }
//!
//!         builder.switch_to_block(block2);
//!         builder.seal_block(block2);
//!
//!         {
//!             let arg1 = builder.use_var(y);
//!             let arg2 = builder.use_var(x);
//!             let tmp = builder.ins().isub(arg1, arg2);
//!             builder.def_var(y, tmp);
//!         }
//!         builder.ins().jump(block1, &[]);
//!         builder.seal_block(block1);
//!     }
//!
//!     let flags = settings::Flags::new(&settings::builder());
//!     let res = verify_function(&func, &flags);
//!     println!("{}", func.display(None));
//!     match res {
//!         Ok(_) => {}
//!         Err(err) => panic!("{}", err),
//!     }
//! }
//! ```

#![deny(missing_docs)]

extern crate cretonne;

pub use frontend::{ILBuilder, FunctionBuilder};

mod frontend;
mod ssa;
