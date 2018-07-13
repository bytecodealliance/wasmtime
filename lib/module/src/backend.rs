//! Defines the `Backend` trait.

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir};
use std::marker;
use DataContext;
use Linkage;
use ModuleNamespace;
use ModuleResult;

/// A `Backend` implements the functionality needed to support a `Module`.
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
    fn new(Self::Builder) -> Self;

    /// Return the `TargetIsa` to compile for.
    fn isa(&self) -> &TargetIsa;

    /// Declare a function.
    fn declare_function(&mut self, name: &str, linkage: Linkage);

    /// Declare a data object.
    fn declare_data(&mut self, name: &str, linkage: Linkage, writable: bool);

    /// Define a function, producing the function body from the given `Context`.
    ///
    /// Functions must be declared before being defined.
    fn define_function(
        &mut self,
        name: &str,
        ctx: &Context,
        namespace: &ModuleNamespace<Self>,
        code_size: u32,
    ) -> ModuleResult<Self::CompiledFunction>;

    /// Define a zero-initialized data object of the given size.
    ///
    /// Data objects must be declared before being defined.
    fn define_data(
        &mut self,
        name: &str,
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
    fn finalize_function(
        &mut self,
        func: &Self::CompiledFunction,
        namespace: &ModuleNamespace<Self>,
    ) -> Self::FinalizedFunction;

    /// Perform all outstanding relocations on the given data object. This requires all
    /// `Local` and `Export` entities referenced to be defined.
    fn finalize_data(
        &mut self,
        data: &Self::CompiledData,
        namespace: &ModuleNamespace<Self>,
    ) -> Self::FinalizedData;

    /// Consume this `Backend` and return a result. Some implementations may
    /// provide additional functionality through this result.
    fn finish(self) -> Self::Product;
}
