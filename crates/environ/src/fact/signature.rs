//! Size, align, and flattening information about component model types.

use crate::component::{ComponentTypes, InterfaceType, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS};
use crate::fact::{AdapterOptions, Context, Options};
use wasm_encoder::ValType;
use wasmtime_component_util::FlagsSize;

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

impl ComponentTypes {
    /// Calculates the core wasm function signature for the component function
    /// type specified within `Context`.
    ///
    /// This is used to generate the core wasm signatures for functions that are
    /// imported (matching whatever was `canon lift`'d) and functions that are
    /// exported (matching the generated function from `canon lower`).
    pub(super) fn signature(&self, options: &AdapterOptions, context: Context) -> Signature {
        let ty = &self[options.ty];
        let ptr_ty = options.options.ptr();

        let mut params = self.flatten_types(&options.options, ty.params.iter().map(|(_, ty)| *ty));
        let mut params_indirect = false;
        if params.len() > MAX_FLAT_PARAMS {
            params = vec![ptr_ty];
            params_indirect = true;
        }

        let mut results = self.flatten_types(&options.options, [ty.result]);
        let mut results_indirect = false;
        if results.len() > MAX_FLAT_RESULTS {
            results_indirect = true;
            match context {
                // For a lifted function too-many-results gets translated to a
                // returned pointer where results are read from. The callee
                // allocates space here.
                Context::Lift => results = vec![ptr_ty],
                // For a lowered function too-many-results becomes a return
                // pointer which is passed as the last argument. The caller
                // allocates space here.
                Context::Lower => {
                    results.truncate(0);
                    params.push(ptr_ty);
                }
            }
        }
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
        tys: impl IntoIterator<Item = InterfaceType>,
    ) -> Vec<ValType> {
        let mut result = Vec::new();
        for ty in tys {
            self.push_flat(opts, &ty, &mut result);
        }
        result
    }

    fn push_flat(&self, opts: &Options, ty: &InterfaceType, dst: &mut Vec<ValType>) {
        match ty {
            InterfaceType::Unit => {}

            InterfaceType::Bool
            | InterfaceType::S8
            | InterfaceType::U8
            | InterfaceType::S16
            | InterfaceType::U16
            | InterfaceType::S32
            | InterfaceType::U32
            | InterfaceType::Char => dst.push(ValType::I32),

            InterfaceType::S64 | InterfaceType::U64 => dst.push(ValType::I64),

            InterfaceType::Float32 => dst.push(ValType::F32),
            InterfaceType::Float64 => dst.push(ValType::F64),

            InterfaceType::String | InterfaceType::List(_) => {
                dst.push(opts.ptr());
                dst.push(opts.ptr());
            }
            InterfaceType::Record(r) => {
                for field in self[*r].fields.iter() {
                    self.push_flat(opts, &field.ty, dst);
                }
            }
            InterfaceType::Tuple(t) => {
                for ty in self[*t].types.iter() {
                    self.push_flat(opts, ty, dst);
                }
            }
            InterfaceType::Flags(f) => {
                let flags = &self[*f];
                match FlagsSize::from_count(flags.names.len()) {
                    FlagsSize::Size0 => {}
                    FlagsSize::Size1 | FlagsSize::Size2 => dst.push(ValType::I32),
                    FlagsSize::Size4Plus(n) => {
                        dst.extend((0..n).map(|_| ValType::I32));
                    }
                }
            }
            InterfaceType::Enum(_) => dst.push(ValType::I32),
            InterfaceType::Option(t) => {
                dst.push(ValType::I32);
                self.push_flat(opts, &self[*t].ty, dst);
            }
            InterfaceType::Variant(t) => {
                dst.push(ValType::I32);
                let pos = dst.len();
                let mut tmp = Vec::new();
                for case in self[*t].cases.iter() {
                    self.push_flat_variant(opts, &case.ty, pos, &mut tmp, dst);
                }
            }
            InterfaceType::Union(t) => {
                dst.push(ValType::I32);
                let pos = dst.len();
                let mut tmp = Vec::new();
                for ty in self[*t].types.iter() {
                    self.push_flat_variant(opts, ty, pos, &mut tmp, dst);
                }
            }
            InterfaceType::Expected(t) => {
                dst.push(ValType::I32);
                let e = &self[*t];
                let pos = dst.len();
                let mut tmp = Vec::new();
                self.push_flat_variant(opts, &e.ok, pos, &mut tmp, dst);
                self.push_flat_variant(opts, &e.err, pos, &mut tmp, dst);
            }
        }
    }

    fn push_flat_variant(
        &self,
        opts: &Options,
        ty: &InterfaceType,
        pos: usize,
        tmp: &mut Vec<ValType>,
        dst: &mut Vec<ValType>,
    ) {
        tmp.truncate(0);
        self.push_flat(opts, ty, tmp);
        for (i, a) in tmp.iter().enumerate() {
            match dst.get_mut(pos + i) {
                Some(b) => join(*a, b),
                None => dst.push(*a),
            }
        }

        fn join(a: ValType, b: &mut ValType) {
            if a == *b {
                return;
            }
            match (a, *b) {
                (ValType::I32, ValType::F32) | (ValType::F32, ValType::I32) => *b = ValType::I32,
                _ => *b = ValType::I64,
            }
        }
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
