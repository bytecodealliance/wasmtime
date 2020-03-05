//! Defines the `Backend` trait.

use crate::DataContext;
use crate::DataId;
use crate::FuncId;
use crate::Linkage;
use crate::ModuleNamespace;
use crate::ModuleResult;
use crate::TrapSite;
use core::marker;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir};

use std::borrow::ToOwned;
use std::boxed::Box;
use std::string::String;
use std::vec::Vec;

/// A `Backend` implements the functionality needed to support a `Module`.
///
/// Three notable implementations of this trait are:
///  - `SimpleJITBackend`, defined in [cranelift-simplejit], which JITs
///    the contents of a `Module` to memory which can be directly executed.
///  - `ObjectBackend`, defined in [cranelift-object], which writes the
///    contents of a `Module` out as a native object file.
///  - `FaerieBackend`, defined in [cranelift-faerie], which writes the
///    contents of a `Module` out as a native object file.
///
/// [cranelift-simplejit]: https://docs.rs/cranelift-simplejit/
/// [cranelift-object]: https://docs.rs/cranelift-object/
/// [cranelift-faerie]: https://docs.rs/cranelift-faerie/
pub trait Backend
where
    Self: marker::Sized,
{
    /// A builder for constructing `Backend` instances.
    type Builder;

    /// The results of compiling a function.
    type CompiledFunction;

    /// The results of "compiling" a data object.
    type CompiledData;

    /// The completed output artifact for a function, if this is meaningful for
    /// the `Backend`.
    type FinalizedFunction;

    /// The completed output artifact for a data object, if this is meaningful for
    /// the `Backend`.
    type FinalizedData;

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
    fn define_function(
        &mut self,
        id: FuncId,
        name: &str,
        ctx: &Context,
        namespace: &ModuleNamespace<Self>,
        code_size: u32,
    ) -> ModuleResult<(Self::CompiledFunction, &[TrapSite])>;

    /// Define a function, taking the function body from the given `bytes`.
    ///
    /// Functions must be declared before being defined.
    fn define_function_bytes(
        &mut self,
        id: FuncId,
        name: &str,
        bytes: &[u8],
        namespace: &ModuleNamespace<Self>,
        traps: Vec<TrapSite>,
    ) -> ModuleResult<(Self::CompiledFunction, &[TrapSite])>;

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
        namespace: &ModuleNamespace<Self>,
    ) -> ModuleResult<Self::CompiledData>;

    /// Write the address of `what` into the data for `data` at `offset`. `data` must refer to a
    /// defined data object.
    fn write_data_funcaddr(
        &mut self,
        data: &mut Self::CompiledData,
        offset: usize,
        what: ir::FuncRef,
    );

    /// Write the address of `what` plus `addend` into the data for `data` at `offset`. `data` must
    /// refer to a defined data object.
    fn write_data_dataaddr(
        &mut self,
        data: &mut Self::CompiledData,
        offset: usize,
        what: ir::GlobalValue,
        addend: binemit::Addend,
    );

    /// Perform all outstanding relocations on the given function. This requires all `Local`
    /// and `Export` entities referenced to be defined.
    ///
    /// This method is not relevant for `Backend` implementations that do not provide
    /// `Backend::FinalizedFunction`.
    fn finalize_function(
        &mut self,
        id: FuncId,
        func: &Self::CompiledFunction,
        namespace: &ModuleNamespace<Self>,
    ) -> Self::FinalizedFunction;

    /// Return the finalized artifact from the backend, if relevant.
    fn get_finalized_function(&self, func: &Self::CompiledFunction) -> Self::FinalizedFunction;

    /// Perform all outstanding relocations on the given data object. This requires all
    /// `Local` and `Export` entities referenced to be defined.
    ///
    /// This method is not relevant for `Backend` implementations that do not provide
    /// `Backend::FinalizedData`.
    fn finalize_data(
        &mut self,
        id: DataId,
        data: &Self::CompiledData,
        namespace: &ModuleNamespace<Self>,
    ) -> Self::FinalizedData;

    /// Return the finalized artifact from the backend, if relevant.
    fn get_finalized_data(&self, data: &Self::CompiledData) -> Self::FinalizedData;

    /// "Publish" all finalized functions and data objects to their ultimate destinations.
    ///
    /// This method is not relevant for `Backend` implementations that do not provide
    /// `Backend::FinalizedFunction` or `Backend::FinalizedData`.
    fn publish(&mut self);

    /// Consume this `Backend` and return a result. Some implementations may
    /// provide additional functionality through this result.
    fn finish(self, namespace: &ModuleNamespace<Self>) -> Self::Product;
}

/// Default names for `ir::LibCall`s. A function by this name is imported into the object as
/// part of the translation of a `ir::ExternalName::LibCall` variant.
pub fn default_libcall_names() -> Box<dyn Fn(ir::LibCall) -> String> {
    Box::new(move |libcall| match libcall {
        ir::LibCall::Probestack => "__cranelift_probestack".to_owned(),
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
