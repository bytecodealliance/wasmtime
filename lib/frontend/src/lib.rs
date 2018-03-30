//! Cretonne IR builder library.
//!
//! Provides a straightforward way to create a Cretonne IR function and fill it with instructions
//! translated from another language. Contains an SSA construction module that lets you translate
//! your non-SSA variables into SSA Cretonne IR values via `use_var` and `def_var` calls.
//!
//! To get started, create an [`FunctionBuilderContext`](struct.FunctionBuilderContext.html) and
//! pass it as an argument to a [`FunctionBuilder`](struct.FunctionBuilder.html).
//!
//! # Example
//!
//! Here is a pseudo-program we want to transform into Cretonne IR:
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
//! Here is how you build the corresponding Cretonne IR function using `FunctionBuilderContext`:
//!
//! ```rust
//! extern crate cretonne;
//! extern crate cton_frontend;
//!
//! use cretonne::entity::EntityRef;
//! use cretonne::ir::{ExternalName, CallConv, Function, Signature, AbiParam, InstBuilder};
//! use cretonne::ir::types::*;
//! use cretonne::settings;
//! use cton_frontend::{FunctionBuilderContext, FunctionBuilder, Variable};
//! use cretonne::verifier::verify_function;
//!
//! fn main() {
//!     let mut sig = Signature::new(CallConv::SystemV);
//!     sig.returns.push(AbiParam::new(I32));
//!     sig.params.push(AbiParam::new(I32));
//!     let mut fn_builder_ctx = FunctionBuilderContext::<Variable>::new();
//!     let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig);
//!     {
//!         let mut builder = FunctionBuilder::<Variable>::new(&mut func, &mut fn_builder_ctx);
//!
//!         let block0 = builder.create_ebb();
//!         let block1 = builder.create_ebb();
//!         let block2 = builder.create_ebb();
//!         let x = Variable::new(0);
//!         let y = Variable::new(1);
//!         let z = Variable::new(2);
//!         builder.declare_var(x, I32);
//!         builder.declare_var(y, I32);
//!         builder.declare_var(z, I32);
//!         builder.append_ebb_params_for_function_params(block0);
//!
//!         builder.switch_to_block(block0);
//!         builder.seal_block(block0);
//!         {
//!             let tmp = builder.ebb_params(block0)[0]; // the first function parameter
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
//!
//!         builder.finalize();
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

#![deny(missing_docs,
        trivial_numeric_casts,
        unused_extern_crates)]

#![cfg_attr(feature="cargo-clippy",
            allow(new_without_default, redundant_field_names))]

extern crate cretonne;

pub use frontend::{FunctionBuilderContext, FunctionBuilder};
pub use variable::Variable;

mod frontend;
mod ssa;
mod variable;
