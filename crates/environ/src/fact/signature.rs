//! Size, align, and flattening information about component model types.

use crate::component::{ComponentTypesBuilder, InterfaceType, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS};
use crate::fact::{AdapterOptions, Context, Options};
use wasm_encoder::ValType;

/// Metadata about a core wasm signature which is created for a component model
/// signature.
#[derive(Debug)]
pub struct Signature {
    /// Core wasm parameters.
    pub params: Vec<ValType>,
    /// Core wasm results.
    pub results: Vec<ValType>,
    /// Indicator to whether parameters are indirect, meaning that the first
    /// entry of `params` is a pointer type which all parameters are loaded
    /// through.
    pub params_indirect: bool,
    /// Indicator whether results are passed indirectly. This may mean that
    /// `results` is an `i32` or that `params` ends with an `i32` depending on
    /// the `Context`.
    pub results_indirect: bool,
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

        let mut params_indirect = false;
        let mut params = match self.flatten_types(
            &options.options,
            MAX_FLAT_PARAMS,
            ty.params.iter().copied(),
        ) {
            Some(list) => list,
            None => {
                params_indirect = true;
                vec![ptr_ty]
            }
        };

        let mut results_indirect = false;
        let results = match self.flatten_types(
            &options.options,
            MAX_FLAT_RESULTS,
            ty.results.iter().map(|ty| *ty),
        ) {
            Some(list) => list,
            None => {
                results_indirect = true;
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
        Signature {
            params,
            results,
            params_indirect,
            results_indirect,
        }
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
}
