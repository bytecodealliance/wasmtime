//! Size, align, and flattening information about component model types.

use crate::component::{InterfaceType, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS};
use crate::fact::{Context, Module, Options};
use wasm_encoder::ValType;
use wasmtime_component_util::{DiscriminantSize, FlagsSize};

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

pub(crate) fn align_to(n: usize, align: usize) -> usize {
    assert!(align.is_power_of_two());
    (n + (align - 1)) & !(align - 1)
}

impl Module<'_> {
    /// Calculates the core wasm function signature for the component function
    /// type specified within `Context`.
    ///
    /// This is used to generate the core wasm signatures for functions that are
    /// imported (matching whatever was `canon lift`'d) and functions that are
    /// exported (matching the generated function from `canon lower`).
    pub(super) fn signature(&self, options: &Options, context: Context) -> Signature {
        let ty = &self.types[options.ty];
        let ptr_ty = options.ptr();

        let mut params = self.flatten_types(options, ty.params.iter().map(|(_, ty)| *ty));
        let mut params_indirect = false;
        if params.len() > MAX_FLAT_PARAMS {
            params = vec![ptr_ty];
            params_indirect = true;
        }

        let mut results = self.flatten_types(options, [ty.result]);
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
                for field in self.types[*r].fields.iter() {
                    self.push_flat(opts, &field.ty, dst);
                }
            }
            InterfaceType::Tuple(t) => {
                for ty in self.types[*t].types.iter() {
                    self.push_flat(opts, ty, dst);
                }
            }
            InterfaceType::Flags(f) => {
                let flags = &self.types[*f];
                let nflags = align_to(flags.names.len(), 32) / 32;
                for _ in 0..nflags {
                    dst.push(ValType::I32);
                }
            }
            InterfaceType::Enum(_) => dst.push(ValType::I32),
            InterfaceType::Option(t) => {
                dst.push(ValType::I32);
                self.push_flat(opts, &self.types[*t], dst);
            }
            InterfaceType::Variant(t) => {
                dst.push(ValType::I32);
                let pos = dst.len();
                let mut tmp = Vec::new();
                for case in self.types[*t].cases.iter() {
                    self.push_flat_variant(opts, &case.ty, pos, &mut tmp, dst);
                }
            }
            InterfaceType::Union(t) => {
                dst.push(ValType::I32);
                let pos = dst.len();
                let mut tmp = Vec::new();
                for ty in self.types[*t].types.iter() {
                    self.push_flat_variant(opts, ty, pos, &mut tmp, dst);
                }
            }
            InterfaceType::Expected(t) => {
                dst.push(ValType::I32);
                let e = &self.types[*t];
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

    pub(super) fn align(&self, opts: &Options, ty: &InterfaceType) -> usize {
        self.size_align(opts, ty).1
    }

    /// Returns a (size, align) pair corresponding to the byte-size and
    /// byte-alignment of the type specified.
    //
    // TODO: this is probably inefficient to entire recalculate at all phases,
    // seems like it would be best to intern this in some sort of map somewhere.
    pub(super) fn size_align(&self, opts: &Options, ty: &InterfaceType) -> (usize, usize) {
        match ty {
            InterfaceType::Unit => (0, 1),
            InterfaceType::Bool | InterfaceType::S8 | InterfaceType::U8 => (1, 1),
            InterfaceType::S16 | InterfaceType::U16 => (2, 2),
            InterfaceType::S32
            | InterfaceType::U32
            | InterfaceType::Char
            | InterfaceType::Float32 => (4, 4),
            InterfaceType::S64 | InterfaceType::U64 | InterfaceType::Float64 => (8, 8),
            InterfaceType::String | InterfaceType::List(_) => {
                ((2 * opts.ptr_size()).into(), opts.ptr_size().into())
            }

            InterfaceType::Record(r) => {
                self.record_size_align(opts, self.types[*r].fields.iter().map(|f| &f.ty))
            }
            InterfaceType::Tuple(t) => self.record_size_align(opts, self.types[*t].types.iter()),
            InterfaceType::Flags(f) => match FlagsSize::from_count(self.types[*f].names.len()) {
                FlagsSize::Size0 => (0, 1),
                FlagsSize::Size1 => (1, 1),
                FlagsSize::Size2 => (2, 2),
                FlagsSize::Size4Plus(n) => (n * 4, 4),
            },
            InterfaceType::Enum(t) => self.discrim_size_align(self.types[*t].names.len()),
            InterfaceType::Option(t) => {
                let ty = &self.types[*t];
                self.variant_size_align(opts, [&InterfaceType::Unit, ty].into_iter())
            }
            InterfaceType::Variant(t) => {
                self.variant_size_align(opts, self.types[*t].cases.iter().map(|c| &c.ty))
            }
            InterfaceType::Union(t) => self.variant_size_align(opts, self.types[*t].types.iter()),
            InterfaceType::Expected(t) => {
                let e = &self.types[*t];
                self.variant_size_align(opts, [&e.ok, &e.err].into_iter())
            }
        }
    }

    pub(super) fn record_size_align<'a>(
        &self,
        opts: &Options,
        fields: impl Iterator<Item = &'a InterfaceType>,
    ) -> (usize, usize) {
        let mut size = 0;
        let mut align = 1;
        for ty in fields {
            let (fsize, falign) = self.size_align(opts, ty);
            size = align_to(size, falign) + fsize;
            align = align.max(falign);
        }
        (align_to(size, align), align)
    }

    fn variant_size_align<'a>(
        &self,
        opts: &Options,
        cases: impl ExactSizeIterator<Item = &'a InterfaceType>,
    ) -> (usize, usize) {
        let (discrim_size, mut align) = self.discrim_size_align(cases.len());
        let mut payload_size = 0;
        for ty in cases {
            let (csize, calign) = self.size_align(opts, ty);
            payload_size = payload_size.max(csize);
            align = align.max(calign);
        }
        (align_to(discrim_size, align) + payload_size, align)
    }

    fn discrim_size_align<'a>(&self, cases: usize) -> (usize, usize) {
        match DiscriminantSize::from_count(cases) {
            Some(DiscriminantSize::Size1) => (1, 1),
            Some(DiscriminantSize::Size2) => (2, 2),
            Some(DiscriminantSize::Size4) => (4, 4),
            None => unreachable!(),
        }
    }
}
