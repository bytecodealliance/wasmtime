//! Size, align, and flattening information about component model types.

use crate::component::{
    ComponentTypesBuilder, InterfaceType, MAX_FLAT_ASYNC_PARAMS, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS,
};
use crate::fact::core_types::CoreTypes;
use crate::fact::{AdapterOptions, DataModel, Options};
use crate::{WasmValType, prelude::*};
use wasm_encoder::ValType;

use super::LinearMemoryOptions;

/// Which half of a function's signature a list of component types is.
///
/// Under the GC data model the flattening comes from the declared
/// `core-type`, so we need to know which part to look at.
#[derive(Copy, Clone)]
pub enum SignaturePart {
    /// Flattening parameters.
    Params,

    /// Flattening results.
    Results,

    /// `Neither` is for when we aren't using translating to/from a `core-type`
    /// signature, such as with helper functions; see also `gc_flat_types`.
    Neither,
}

/// Metadata about a core wasm signature which is created for a component model
/// signature.
#[derive(Debug)]
pub struct Signature {
    /// Core wasm parameters.
    pub params: Vec<ValType>,
    /// Core wasm results.
    pub results: Vec<ValType>,
}

impl ComponentTypesBuilder {
    /// Calculates the core wasm function signature for the component function
    /// type specified within `Context`.
    ///
    /// This is used to generate the core wasm signatures for functions that are
    /// imported (matching whatever was `canon lift`'d) and functions that are
    /// exported (matching the generated function from `canon lower`).
    pub(super) fn signature(&self, core_types: &mut CoreTypes, options: &Options) -> Signature {
        let f = &self.module_types_builder()[options.core_type]
            .composite_type
            .inner
            .unwrap_func();
        Signature {
            params: f
                .params()
                .iter()
                .map(|ty| self.val_type(core_types, ty))
                .collect(),
            results: f
                .results()
                .iter()
                .map(|ty| self.val_type(core_types, ty))
                .collect(),
        }
    }

    fn val_type(&self, core_types: &mut CoreTypes, ty: &WasmValType) -> ValType {
        match ty {
            WasmValType::I32 => ValType::I32,
            WasmValType::I64 => ValType::I64,
            WasmValType::F32 => ValType::F32,
            WasmValType::F64 => ValType::F64,
            WasmValType::V128 => ValType::V128,

            // The adapter must name the same type that the caller and callee
            // use, which means cloning the rec group it was defined in into
            // this adapter module and rewriting the reference to point at the
            // clone.
            WasmValType::Ref(r) => {
                let types = self.module_types_builder().types();
                if let Some(ty) = r.heap_type.as_concrete_type_index() {
                    let ty = ty.unwrap_module_type_index();
                    core_types.rec_group(types, types.rec_group_of(ty));
                }
                ValType::Ref(core_types.ref_type(types, r, &None))
            }
        }
    }

    /// Generates the signature for a function to be exported by the adapter
    /// module and called by the host to lift the parameters from the caller and
    /// lower them to the callee.
    ///
    /// This allows the host to delay copying the parameters until the callee
    /// signals readiness by clearing its backpressure flag.
    ///
    /// Note that this function uses multi-value return to return up to
    /// `MAX_FLAT_PARAMS` _results_ via the stack, allowing the host to pass
    /// them directly to the callee with no additional effort.
    pub(super) fn async_start_signature(
        &self,
        core_types: &mut CoreTypes,
        lower: &AdapterOptions,
        lift: &AdapterOptions,
    ) -> Signature {
        let lower_ty = &self[lower.ty];
        let lower_ptr_ty = lower.options.data_model.unwrap_memory().ptr();
        let max_flat_params = if lower.options.async_ {
            MAX_FLAT_ASYNC_PARAMS
        } else {
            MAX_FLAT_PARAMS
        };
        let params = match self.flatten_types(
            core_types,
            &lower.options,
            SignaturePart::Params,
            max_flat_params,
            self[lower_ty.params].types.iter().copied(),
        ) {
            Some(list) => list,
            None => vec![lower_ptr_ty],
        };

        let lift_ty = &self[lift.ty];
        let lift_ptr_ty = lift.options.data_model.unwrap_memory().ptr();
        let results = match self.flatten_types(
            core_types,
            &lift.options,
            SignaturePart::Params,
            // Both sync- and async-lifted functions accept up to this many core
            // parameters via the stack.  The host will call the `async-start`
            // function (possibly after a backpressure delay), which will
            // _return_ that many values (using a multi-value return, if
            // necessary); the host will then pass them directly to the callee.
            MAX_FLAT_PARAMS,
            self[lift_ty.params].types.iter().copied(),
        ) {
            Some(list) => list,
            None => {
                vec![lift_ptr_ty]
            }
        };

        Signature { params, results }
    }

    pub(super) fn flatten_lowering_types(
        &self,
        core_types: &mut CoreTypes,
        options: &Options,
        tys: impl IntoIterator<Item = InterfaceType>,
    ) -> Option<Vec<ValType>> {
        // Async functions "use the stack" for zero return values, meaning
        // nothing is actually passed, but otherwise if anything is returned
        // it's always through memory.
        let max = if options.async_ { 0 } else { MAX_FLAT_RESULTS };
        self.flatten_types(core_types, options, SignaturePart::Results, max, tys)
    }

    pub(super) fn flatten_lifting_types(
        &self,
        core_types: &mut CoreTypes,
        options: &Options,
        tys: impl IntoIterator<Item = InterfaceType>,
    ) -> Option<Vec<ValType>> {
        self.flatten_types(
            core_types,
            options,
            SignaturePart::Results,
            if options.async_ {
                // Async functions return results by calling `task.return`,
                // which accepts up to `MAX_FLAT_PARAMS` parameters via the
                // stack.
                MAX_FLAT_PARAMS
            } else {
                // Sync functions return results directly (at least until we add
                // a `always-task-return` canonical option) and so are limited
                // to returning up to `MAX_FLAT_RESULTS` results via the stack.
                MAX_FLAT_RESULTS
            },
            tys,
        )
    }

    /// Generates the signature for a function to be exported by the adapter
    /// module and called by the host to lift the results from the callee and
    /// lower them to the caller.
    ///
    /// Given that async-lifted exports return their results via the
    /// `task.return` intrinsic, the host will need to copy the results from
    /// callee to caller when that intrinsic is called rather than when the
    /// callee task fully completes (which may happen much later).
    pub(super) fn async_return_signature(
        &self,
        core_types: &mut CoreTypes,
        lower: &AdapterOptions,
        lift: &AdapterOptions,
    ) -> Signature {
        let lift_ty = &self[lift.ty];
        let lift_ptr_ty = lift.options.data_model.unwrap_memory().ptr();
        let mut params = match self.flatten_lifting_types(
            core_types,
            &lift.options,
            self[lift_ty.results].types.iter().copied(),
        ) {
            Some(list) => list,
            None => {
                vec![lift_ptr_ty]
            }
        };

        let lower_ty = &self[lower.ty];
        let lower_result_tys = &self[lower_ty.results];
        let results = if lower.options.async_ {
            // Add return pointer
            //
            // FIXME(#4311): the return pointer points into the caller's
            // memory, so this should be the lower side's pointer type.
            if !lower_result_tys.types.is_empty() {
                params.push(lift_ptr_ty);
            }
            Vec::new()
        } else {
            match self.flatten_types(
                core_types,
                &lower.options,
                SignaturePart::Results,
                MAX_FLAT_RESULTS,
                lower_result_tys.types.iter().copied(),
            ) {
                Some(list) => list,
                None => {
                    // Add return pointer
                    //
                    // FIXME(#4311): the return pointer points into the
                    // caller's memory, so this should be the lower side's
                    // pointer type.
                    params.push(lift_ptr_ty);
                    Vec::new()
                }
            }
        };

        Signature { params, results }
    }

    /// Pushes the flat version of a list of component types into a final result
    /// list.
    pub(super) fn flatten_types(
        &self,
        core_types: &mut CoreTypes,
        opts: &Options,
        which: SignaturePart,
        max: usize,
        tys: impl IntoIterator<Item = InterfaceType>,
    ) -> Option<Vec<ValType>> {
        let tys = tys.into_iter().collect::<Vec<_>>();

        match (&opts.data_model, which) {
            // GC never spills, so `max` does not apply and this cannot fail.
            (DataModel::Gc {}, SignaturePart::Params | SignaturePart::Results) => {
                Some(self.gc_flat_types(core_types, opts, which, tys.len()))
            }
            (_, _) => {
                let mut dst = Vec::new();
                for ty in tys {
                    for ty in self.flat_val_types(opts, &ty)? {
                        if dst.len() >= max {
                            return None;
                        }
                        dst.push(ty);
                    }
                }
                Some(dst)
            }
        }
    }

    /// The core value types that `ty` flattens to under `opts`.
    ///
    /// Works one type at a time, so it only reports what `FlatType` can
    /// describe; see `Options::scalar_flat_types`.
    fn flat_val_types(&self, opts: &Options, ty: &InterfaceType) -> Option<Vec<ValType>> {
        match opts.scalar_flat_types(ty, self) {
            Some(flat) => Some(flat.iter().map(|ty| (*ty).into()).collect()),

            None => match opts.data_model {
                // `flatten_types` routes GC parameter and result lists to
                // `gc_flat_types`, so only a `SignaturePart::Neither` helper
                // lands here, which no GC-supported type reaches today.
                DataModel::Gc {} => {
                    todo!("CM+GC: helper function for a reference-lowered type")
                }
                DataModel::LinearMemory(_) => None,
            },
        }
    }

    /// Flatten a parameter or result list under the GC data model.
    ///
    /// The `core-type` canonical option declares the core signature up front,
    /// and validation enforces that it maps 1:1 to the component signature, so
    /// the declared types are the flattening. This is the only way to flatten
    /// a type that lowers to a reference, which `FlatType` cannot describe.
    /// Nothing spills to memory under this data model, so this cannot fail.
    ///
    /// See https://github.com/WebAssembly/component-model/issues/525.
    fn gc_flat_types(
        &self,
        core_types: &mut CoreTypes,
        opts: &Options,
        which: SignaturePart,
        len: usize,
    ) -> Vec<ValType> {
        let sig = self.signature(core_types, opts);
        let flat = match which {
            SignaturePart::Params => sig.params,
            SignaturePart::Results => sig.results,
            SignaturePart::Neither => unreachable!("not a parameter or result list"),
        };
        assert_eq!(
            flat.len(),
            len,
            "the declared `core-type` must map 1:1 to the component signature",
        );
        flat
    }

    pub(super) fn align(&self, opts: &LinearMemoryOptions, ty: &InterfaceType) -> u32 {
        self.size_align(opts, ty).1
    }

    /// Returns a (size, align) pair corresponding to the byte-size and
    /// byte-alignment of the type specified.
    //
    // TODO: this is probably inefficient to entire recalculate at all phases,
    // seems like it would be best to intern this in some sort of map somewhere.
    pub(super) fn size_align(&self, opts: &LinearMemoryOptions, ty: &InterfaceType) -> (u32, u32) {
        let abi = self.canonical_abi(ty);
        if opts.memory64() {
            (abi.size64, abi.align64)
        } else {
            (abi.size32, abi.align32)
        }
    }

    /// Tests whether the type signature for `options` contains a borrowed
    /// resource anywhere.
    pub(super) fn contains_borrow_resource(&self, options: &AdapterOptions) -> bool {
        let ty = &self[options.ty];

        // Only parameters need to be checked since results should never have
        // borrowed resources.
        debug_assert!(
            !self[ty.results]
                .types
                .iter()
                .any(|t| self.ty_contains_borrow_resource(t))
        );
        self[ty.params]
            .types
            .iter()
            .any(|t| self.ty_contains_borrow_resource(t))
    }
}
