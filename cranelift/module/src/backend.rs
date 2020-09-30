//! Defines the `Backend` trait.

use crate::DataId;
use crate::FuncId;
use crate::Linkage;
use crate::ModuleDeclarations;
use crate::ModuleResult;
use crate::{DataContext, FuncOrDataId};
use core::marker;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir};

use std::boxed::Box;
use std::string::String;
use std::{borrow::ToOwned, collections::HashMap};

/// A `Backend` implements the functionality needed to support a `Module`.
///
/// Three notable implementations of this trait are:
///  - `SimpleJITBackend`, defined in [cranelift-simplejit], which JITs
///    the contents of a `Module` to memory which can be directly executed.
///  - `ObjectBackend`, defined in [cranelift-object], which writes the
///    contents of a `Module` out as a native object file.
///
/// [cranelift-simplejit]: https://docs.rs/cranelift-simplejit/
/// [cranelift-object]: https://docs.rs/cranelift-object/
pub trait Backend
where
    Self: marker::Sized,
{
    /// A builder for constructing `Backend` instances.
    type Builder;

    /// This is an object returned by `Module`'s
    /// [`finish`](struct.Module.html#method.finish) function,
    /// if the `Backend` has a purpose for this.
    type Product;

    /// Create a new `Backend` instance.
    fn new(_: Self::Builder) -> Self;

    /// Return the `TargetIsa` to compile for.
    fn isa(&self) -> &dyn TargetIsa;

    /// Declare a function.
    fn declare_function(&mut self, id: FuncId, name: &str, linkage: Linkage);

    /// Declare a data object.
    fn declare_data(
        &mut self,
        id: DataId,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
        align: Option<u8>,
    );

    /// Define a function, producing the function body from the given `Context`.
    ///
    /// Functions must be declared before being defined.
    fn define_function<TS>(
        &mut self,
        id: FuncId,
        name: &str,
        ctx: &Context,
        declarations: &ModuleDeclarations,
        code_size: u32,
        trap_sink: &mut TS,
    ) -> ModuleResult<()>
    where
        TS: binemit::TrapSink;

    /// Define a function, taking the function body from the given `bytes`.
    ///
    /// Functions must be declared before being defined.
    fn define_function_bytes(
        &mut self,
        id: FuncId,
        name: &str,
        bytes: &[u8],
        declarations: &ModuleDeclarations,
    ) -> ModuleResult<()>;

    /// Define a zero-initialized data object of the given size.
    ///
    /// Data objects must be declared before being defined.
    fn define_data(
        &mut self,
        id: DataId,
        name: &str,
        writable: bool,
        tls: bool,
        align: Option<u8>,
        data_ctx: &DataContext,
        declarations: &ModuleDeclarations,
    ) -> ModuleResult<()>;

    /// Consume this `Backend` and return a result. Some implementations may
    /// provide additional functionality through this result.
    fn finish(
        self,
        names: HashMap<String, FuncOrDataId>,
        declarations: ModuleDeclarations,
    ) -> Self::Product;
}

/// Default names for `ir::LibCall`s. A function by this name is imported into the object as
/// part of the translation of a `ir::ExternalName::LibCall` variant.
pub fn default_libcall_names() -> Box<dyn Fn(ir::LibCall) -> String> {
    Box::new(move |libcall| match libcall {
        ir::LibCall::Probestack => "__cranelift_probestack".to_owned(),
        ir::LibCall::UdivI64 => "__udivdi3".to_owned(),
        ir::LibCall::SdivI64 => "__divdi3".to_owned(),
        ir::LibCall::UremI64 => "__umoddi3".to_owned(),
        ir::LibCall::SremI64 => "__moddi3".to_owned(),
        ir::LibCall::IshlI64 => "__ashldi3".to_owned(),
        ir::LibCall::UshrI64 => "__lshrdi3".to_owned(),
        ir::LibCall::SshrI64 => "__ashrdi3".to_owned(),
        ir::LibCall::CeilF32 => "ceilf".to_owned(),
        ir::LibCall::CeilF64 => "ceil".to_owned(),
        ir::LibCall::FloorF32 => "floorf".to_owned(),
        ir::LibCall::FloorF64 => "floor".to_owned(),
        ir::LibCall::TruncF32 => "truncf".to_owned(),
        ir::LibCall::TruncF64 => "trunc".to_owned(),
        ir::LibCall::NearestF32 => "nearbyintf".to_owned(),
        ir::LibCall::NearestF64 => "nearbyint".to_owned(),
        ir::LibCall::Memcpy => "memcpy".to_owned(),
        ir::LibCall::Memset => "memset".to_owned(),
        ir::LibCall::Memmove => "memmove".to_owned(),

        ir::LibCall::ElfTlsGetAddr => "__tls_get_addr".to_owned(),
    })
}
