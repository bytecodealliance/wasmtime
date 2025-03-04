//! Size, align, and flattening information about component model types.

use crate::component::{ComponentTypesBuilder, InterfaceType, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS};
use crate::fact::{AdapterOptions, Context, Options};
use crate::prelude::*;
use wasm_encoder::ValType;

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
    pub(super) fn signature(&self, options: &AdapterOptions, context: Context) -> Signature {
        let ty = &self[options.ty];
        let ptr_ty = options.options.ptr();

        // The async lower ABI is always `(param i32 i32) (result i32)` (for
        // wasm32, anyway), regardless of the component-level signature.
        //
        // The first param is a pointer to linear memory where the parameters have
        // been stored by the caller, the second param is a pointer to linear
        // memory where the results should be stored by the callee, and the
        // result is a status code optionally ORed with a subtask ID.
        if let (Context::Lower, true) = (&context, options.options.async_) {
            return Signature {
                params: vec![ptr_ty; 2],
                results: vec![ValType::I32],
            };
        }

        // If we're lifting async or sync, or if we're lowering sync, we can
        // pass up to `MAX_FLAT_PARAMS` via the stack.
        let mut params = match self.flatten_types(
            &options.options,
            MAX_FLAT_PARAMS,
            self[ty.params].types.iter().copied(),
        ) {
            Some(list) => list,
            None => {
                vec![ptr_ty]
            }
        };

        // If we're lifting async with a callback, the result is an `i32` status
        // code, optionally ORed with a guest task identifier, and the result
        // will be returned via `task.return`.
        //
        // If we're lifting async without a callback, then there's no need to return
        // anything here since the result will be returned via `task.return` and the
        // guest will use `task.wait` rather than return a status code in order to suspend
        // itself, if necessary.
        if options.options.async_ {
            return Signature {
                params,
                results: if options.options.callback.is_some() {
                    vec![ptr_ty]
                } else {
                    Vec::new()
                },
            };
        }

        // If we've reached this point, we're either lifting or lowering sync,
        // in which case the guest will return up to `MAX_FLAT_RESULTS` via the
        // stack or spill to linear memory otherwise.
        let results = match self.flatten_types(
            &options.options,
            MAX_FLAT_RESULTS,
            self[ty.results].types.iter().copied(),
        ) {
            Some(list) => list,
            None => {
                match context {
                    // For a lifted function too-many-results gets translated to a
                    // returned pointer where results are read from. The callee
                    // allocates space here.
                    Context::Lift => vec![ptr_ty],
                    // For a lowered function too-many-results becomes a return
                    // pointer which is passed as the last argument. The caller
                    // allocates space here.
                    Context::Lower => {
                        params.push(ptr_ty);
                        Vec::new()
                    }
                }
            }
        };
        Signature { params, results }
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
        lower: &AdapterOptions,
        lift: &AdapterOptions,
    ) -> Signature {
        let lower_ty = &self[lower.ty];
        let lower_ptr_ty = lower.options.ptr();
        let params = if lower.options.async_ {
            vec![lower_ptr_ty]
        } else {
            match self.flatten_types(
                &lower.options,
                MAX_FLAT_PARAMS,
                self[lower_ty.params].types.iter().copied(),
            ) {
                Some(list) => list,
                None => {
                    vec![lower_ptr_ty]
                }
            }
        };

        let lift_ty = &self[lift.ty];
        let lift_ptr_ty = lift.options.ptr();
        let results = match self.flatten_types(
            &lift.options,
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
        options: &Options,
        tys: impl IntoIterator<Item = InterfaceType>,
    ) -> Option<Vec<ValType>> {
        if options.async_ {
            // When lowering an async function, we always spill parameters to
            // linear memory.
            None
        } else {
            self.flatten_types(options, MAX_FLAT_RESULTS, tys)
        }
    }

    pub(super) fn flatten_lifting_types(
        &self,
        options: &Options,
        tys: impl IntoIterator<Item = InterfaceType>,
    ) -> Option<Vec<ValType>> {
        self.flatten_types(
            options,
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
        lower: &AdapterOptions,
        lift: &AdapterOptions,
    ) -> Signature {
        let lift_ty = &self[lift.ty];
        let lift_ptr_ty = lift.options.ptr();
        let mut params = match self
            .flatten_lifting_types(&lift.options, self[lift_ty.results].types.iter().copied())
        {
            Some(list) => list,
            None => {
                vec![lift_ptr_ty]
            }
        };

        let lower_ty = &self[lower.ty];
        let results = if lower.options.async_ {
            // Add return pointer
            params.push(lift_ptr_ty);
            Vec::new()
        } else {
            match self.flatten_types(
                &lower.options,
                MAX_FLAT_RESULTS,
                self[lower_ty.results].types.iter().copied(),
            ) {
                Some(list) => list,
                None => {
                    // Add return pointer
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
        opts: &Options,
        max: usize,
        tys: impl IntoIterator<Item = InterfaceType>,
    ) -> Option<Vec<ValType>> {
        let mut dst = Vec::new();
        for ty in tys {
            for ty in opts.flat_types(&ty, self)? {
                if dst.len() == max {
                    return None;
                }
                dst.push((*ty).into());
            }
        }
        Some(dst)
    }

    pub(super) fn align(&self, opts: &Options, ty: &InterfaceType) -> u32 {
        self.size_align(opts, ty).1
    }

    /// Returns a (size, align) pair corresponding to the byte-size and
    /// byte-alignment of the type specified.
    //
    // TODO: this is probably inefficient to entire recalculate at all phases,
    // seems like it would be best to intern this in some sort of map somewhere.
    pub(super) fn size_align(&self, opts: &Options, ty: &InterfaceType) -> (u32, u32) {
        let abi = self.canonical_abi(ty);
        if opts.memory64 {
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
        debug_assert!(!self[ty.results]
            .types
            .iter()
            .any(|t| self.ty_contains_borrow_resource(t)));
        self[ty.params]
            .types
            .iter()
            .any(|t| self.ty_contains_borrow_resource(t))
    }
}
