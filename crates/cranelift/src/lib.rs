//! Support for compiling with Cranelift.
//!
//! This crate provides an implementation of the `wasmtime_environ::Compiler`
//! and `wasmtime_environ::CompilerBuilder` traits.

// See documentation in crates/wasmtime/src/runtime.rs for why this is
// selectively enabled here.
#![warn(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use cranelift_codegen::{
    binemit,
    cursor::FuncCursor,
    ir::{self, AbiParam, ArgumentPurpose, ExternalName, InstBuilder, Signature, TrapCode},
    isa::{CallConv, TargetIsa},
    settings, FinalizedMachReloc, FinalizedRelocTarget, MachTrap,
};
use cranelift_entity::PrimaryMap;

use target_lexicon::Architecture;
use wasmtime_environ::{
    BuiltinFunctionIndex, FlagValue, FuncIndex, RelocationTarget, Trap, TrapInformation, Tunables,
    WasmFuncType, WasmHeapTopType, WasmHeapType, WasmValType,
};

pub use builder::builder;

pub mod isa_builder;
mod obj;
pub use obj::*;
mod compiled_function;
pub use compiled_function::*;

mod builder;
mod compiler;
mod debug;
mod func_environ;
mod gc;
mod translate;

const TRAP_INTERNAL_ASSERT: TrapCode = TrapCode::unwrap_user(1);
const TRAP_OFFSET: u8 = 2;
pub const TRAP_ALWAYS: TrapCode =
    TrapCode::unwrap_user(Trap::AlwaysTrapAdapter as u8 + TRAP_OFFSET);
pub const TRAP_CANNOT_ENTER: TrapCode =
    TrapCode::unwrap_user(Trap::CannotEnterComponent as u8 + TRAP_OFFSET);
pub const TRAP_INDIRECT_CALL_TO_NULL: TrapCode =
    TrapCode::unwrap_user(Trap::IndirectCallToNull as u8 + TRAP_OFFSET);
pub const TRAP_BAD_SIGNATURE: TrapCode =
    TrapCode::unwrap_user(Trap::BadSignature as u8 + TRAP_OFFSET);
pub const TRAP_NULL_REFERENCE: TrapCode =
    TrapCode::unwrap_user(Trap::NullReference as u8 + TRAP_OFFSET);
pub const TRAP_ALLOCATION_TOO_LARGE: TrapCode =
    TrapCode::unwrap_user(Trap::AllocationTooLarge as u8 + TRAP_OFFSET);
pub const TRAP_ARRAY_OUT_OF_BOUNDS: TrapCode =
    TrapCode::unwrap_user(Trap::ArrayOutOfBounds as u8 + TRAP_OFFSET);
pub const TRAP_UNREACHABLE: TrapCode =
    TrapCode::unwrap_user(Trap::UnreachableCodeReached as u8 + TRAP_OFFSET);
pub const TRAP_HEAP_MISALIGNED: TrapCode =
    TrapCode::unwrap_user(Trap::HeapMisaligned as u8 + TRAP_OFFSET);
pub const TRAP_TABLE_OUT_OF_BOUNDS: TrapCode =
    TrapCode::unwrap_user(Trap::TableOutOfBounds as u8 + TRAP_OFFSET);
pub const TRAP_CAST_FAILURE: TrapCode =
    TrapCode::unwrap_user(Trap::CastFailure as u8 + TRAP_OFFSET);

/// Creates a new cranelift `Signature` with no wasm params/results for the
/// given calling convention.
///
/// This will add the default vmctx/etc parameters to the signature returned.
fn blank_sig(isa: &dyn TargetIsa, call_conv: CallConv) -> ir::Signature {
    let pointer_type = isa.pointer_type();
    let mut sig = ir::Signature::new(call_conv);
    // Add the caller/callee `vmctx` parameters.
    sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));
    sig.params.push(ir::AbiParam::new(pointer_type));
    return sig;
}

/// Emit code for the following unbarriered memory write of the given type:
///
/// ```ignore
/// *(base + offset) = value
/// ```
///
/// This is intended to be used with things like `ValRaw` and the array calling
/// convention.
fn unbarriered_store_type_at_offset(
    pos: &mut FuncCursor,
    flags: ir::MemFlags,
    base: ir::Value,
    offset: i32,
    value: ir::Value,
) {
    pos.ins().store(flags, value, base, offset);
}

/// Emit code to do the following unbarriered memory read of the given type and
/// with the given flags:
///
/// ```ignore
/// result = *(base + offset)
/// ```
///
/// This is intended to be used with things like `ValRaw` and the array calling
/// convention.
fn unbarriered_load_type_at_offset(
    isa: &dyn TargetIsa,
    pos: &mut FuncCursor,
    ty: WasmValType,
    flags: ir::MemFlags,
    base: ir::Value,
    offset: i32,
) -> ir::Value {
    let ir_ty = value_type(isa, ty);
    pos.ins().load(ir_ty, flags, base, offset)
}

/// Returns the corresponding cranelift type for the provided wasm type.
fn value_type(isa: &dyn TargetIsa, ty: WasmValType) -> ir::types::Type {
    match ty {
        WasmValType::I32 => ir::types::I32,
        WasmValType::I64 => ir::types::I64,
        WasmValType::F32 => ir::types::F32,
        WasmValType::F64 => ir::types::F64,
        WasmValType::V128 => ir::types::I8X16,
        WasmValType::Ref(rt) => reference_type(rt.heap_type, isa.pointer_type()),
    }
}

/// Get the Cranelift signature for all array-call functions, that is:
///
/// ```ignore
/// unsafe extern "C" fn(
///     callee_vmctx: *mut VMOpaqueContext,
///     caller_vmctx: *mut VMOpaqueContext,
///     values_ptr: *mut ValRaw,
///     values_len: usize,
/// )
/// ```
///
/// This signature uses the target's default calling convention.
///
/// Note that regardless of the Wasm function type, the array-call calling
/// convention always uses that same signature.
fn array_call_signature(isa: &dyn TargetIsa) -> ir::Signature {
    let mut sig = blank_sig(isa, CallConv::triple_default(isa.triple()));
    // The array-call signature has an added parameter for the `values_vec`
    // input/output buffer in addition to the size of the buffer, in units
    // of `ValRaw`.
    sig.params.push(ir::AbiParam::new(isa.pointer_type()));
    sig.params.push(ir::AbiParam::new(isa.pointer_type()));
    sig
}

/// Get the internal Wasm calling convention for the target/tunables combo
fn wasm_call_conv(isa: &dyn TargetIsa, tunables: &Tunables) -> CallConv {
    // The default calling convention is `CallConv::Tail` to enable the use of
    // tail calls in modules when needed. Note that this is used even if the
    // tail call proposal is disabled in wasm. This is not interacted with on
    // the host so it's purely an internal detail of wasm itself.
    //
    // The Winch calling convention is used instead when generating trampolines
    // which call Winch-generated functions. The winch calling convention is
    // only implemented for x64 and aarch64, so assert that here and panic on
    // other architectures.
    if tunables.winch_callable {
        assert!(
            matches!(
                isa.triple().architecture,
                Architecture::X86_64 | Architecture::Aarch64(_)
            ),
            "The Winch calling convention is only implemented for x86_64 and aarch64"
        );
        CallConv::Winch
    } else {
        CallConv::Tail
    }
}

/// Get the internal Wasm calling convention signature for the given type.
fn wasm_call_signature(
    isa: &dyn TargetIsa,
    wasm_func_ty: &WasmFuncType,
    tunables: &Tunables,
) -> ir::Signature {
    let call_conv = wasm_call_conv(isa, tunables);
    let mut sig = blank_sig(isa, call_conv);
    let cvt = |ty: &WasmValType| ir::AbiParam::new(value_type(isa, *ty));
    sig.params.extend(wasm_func_ty.params().iter().map(&cvt));
    sig.returns.extend(wasm_func_ty.returns().iter().map(&cvt));
    sig
}

/// Returns the reference type to use for the provided wasm type.
fn reference_type(wasm_ht: WasmHeapType, pointer_type: ir::Type) -> ir::Type {
    match wasm_ht.top() {
        WasmHeapTopType::Func => pointer_type,
        WasmHeapTopType::Any | WasmHeapTopType::Extern => ir::types::I32,
    }
}

// List of namespaces which are processed in `mach_reloc_to_reloc` below.

/// Namespace corresponding to wasm functions, the index is the index of the
/// defined function that's being referenced.
pub const NS_WASM_FUNC: u32 = 0;

/// Namespace for builtin function trampolines. The index is the index of the
/// builtin that's being referenced. These trampolines invoke the real host
/// function through an indirect function call loaded by the `VMContext`.
pub const NS_WASMTIME_BUILTIN: u32 = 1;

/// A record of a relocation to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    /// The relocation code.
    pub reloc: binemit::Reloc,
    /// Relocation target.
    pub reloc_target: RelocationTarget,
    /// The offset where to apply the relocation.
    pub offset: binemit::CodeOffset,
    /// The addend to add to the relocation value.
    pub addend: binemit::Addend,
}

/// Converts cranelift_codegen settings to the wasmtime_environ equivalent.
pub fn clif_flags_to_wasmtime(
    flags: impl IntoIterator<Item = settings::Value>,
) -> Vec<(&'static str, FlagValue<'static>)> {
    flags
        .into_iter()
        .map(|val| (val.name, to_flag_value(&val)))
        .collect()
}

fn to_flag_value(v: &settings::Value) -> FlagValue<'static> {
    match v.kind() {
        settings::SettingKind::Enum => FlagValue::Enum(v.as_enum().unwrap()),
        settings::SettingKind::Num => FlagValue::Num(v.as_num().unwrap()),
        settings::SettingKind::Bool => FlagValue::Bool(v.as_bool().unwrap()),
        settings::SettingKind::Preset => unreachable!(),
    }
}

/// Converts machine traps to trap information.
pub fn mach_trap_to_trap(trap: &MachTrap) -> Option<TrapInformation> {
    let &MachTrap { offset, code } = trap;
    Some(TrapInformation {
        code_offset: offset,
        trap_code: clif_trap_to_env_trap(code)?,
    })
}

fn clif_trap_to_env_trap(trap: ir::TrapCode) -> Option<Trap> {
    Some(match trap {
        ir::TrapCode::STACK_OVERFLOW => Trap::StackOverflow,
        ir::TrapCode::HEAP_OUT_OF_BOUNDS => Trap::MemoryOutOfBounds,
        ir::TrapCode::INTEGER_OVERFLOW => Trap::IntegerOverflow,
        ir::TrapCode::INTEGER_DIVISION_BY_ZERO => Trap::IntegerDivisionByZero,
        ir::TrapCode::BAD_CONVERSION_TO_INTEGER => Trap::BadConversionToInteger,

        // These do not get converted to wasmtime traps, since they
        // shouldn't ever be hit in theory. Instead of catching and handling
        // these, we let the signal crash the process.
        TRAP_INTERNAL_ASSERT => return None,

        other => Trap::from_u8(other.as_raw().get() - TRAP_OFFSET).unwrap(),
    })
}

/// Converts machine relocations to relocation information
/// to perform.
fn mach_reloc_to_reloc(
    reloc: &FinalizedMachReloc,
    name_map: &PrimaryMap<ir::UserExternalNameRef, ir::UserExternalName>,
) -> Relocation {
    let &FinalizedMachReloc {
        offset,
        kind,
        ref target,
        addend,
    } = reloc;
    let reloc_target = match *target {
        FinalizedRelocTarget::ExternalName(ExternalName::User(user_func_ref)) => {
            let name = &name_map[user_func_ref];
            match name.namespace {
                NS_WASM_FUNC => RelocationTarget::Wasm(FuncIndex::from_u32(name.index)),
                NS_WASMTIME_BUILTIN => {
                    RelocationTarget::Builtin(BuiltinFunctionIndex::from_u32(name.index))
                }
                _ => panic!("unknown namespace {}", name.namespace),
            }
        }
        FinalizedRelocTarget::ExternalName(ExternalName::LibCall(libcall)) => {
            let libcall = libcall_cranelift_to_wasmtime(libcall);
            RelocationTarget::HostLibcall(libcall)
        }
        _ => panic!("unrecognized external name"),
    };
    Relocation {
        reloc: kind,
        reloc_target,
        offset,
        addend,
    }
}

fn libcall_cranelift_to_wasmtime(call: ir::LibCall) -> wasmtime_environ::obj::LibCall {
    use wasmtime_environ::obj::LibCall as LC;
    match call {
        ir::LibCall::FloorF32 => LC::FloorF32,
        ir::LibCall::FloorF64 => LC::FloorF64,
        ir::LibCall::NearestF32 => LC::NearestF32,
        ir::LibCall::NearestF64 => LC::NearestF64,
        ir::LibCall::CeilF32 => LC::CeilF32,
        ir::LibCall::CeilF64 => LC::CeilF64,
        ir::LibCall::TruncF32 => LC::TruncF32,
        ir::LibCall::TruncF64 => LC::TruncF64,
        ir::LibCall::FmaF32 => LC::FmaF32,
        ir::LibCall::FmaF64 => LC::FmaF64,
        ir::LibCall::X86Pshufb => LC::X86Pshufb,
        _ => panic!("cranelift emitted a libcall wasmtime does not support: {call:?}"),
    }
}

/// Helper structure for creating a `Signature` for all builtins.
struct BuiltinFunctionSignatures {
    pointer_type: ir::Type,

    #[cfg(feature = "gc")]
    reference_type: ir::Type,

    host_call_conv: CallConv,
    wasm_call_conv: CallConv,
}

impl BuiltinFunctionSignatures {
    fn new(isa: &dyn TargetIsa, tunables: &Tunables) -> Self {
        Self {
            pointer_type: isa.pointer_type(),
            host_call_conv: CallConv::triple_default(isa.triple()),
            wasm_call_conv: wasm_call_conv(isa, tunables),

            #[cfg(feature = "gc")]
            reference_type: ir::types::I32,
        }
    }

    fn vmctx(&self) -> AbiParam {
        AbiParam::special(self.pointer_type, ArgumentPurpose::VMContext)
    }

    #[cfg(feature = "gc")]
    fn reference(&self) -> AbiParam {
        AbiParam::new(self.reference_type)
    }

    fn pointer(&self) -> AbiParam {
        AbiParam::new(self.pointer_type)
    }

    fn i32(&self) -> AbiParam {
        // Some platform ABIs require i32 values to be zero- or sign-
        // extended to the full register width.  We need to indicate
        // this here by using the appropriate .uext or .sext attribute.
        // The attribute can be added unconditionally; platforms whose
        // ABI does not require such extensions will simply ignore it.
        // Note that currently all i32 arguments or return values used
        // by builtin functions are unsigned, so we always use .uext.
        // If that ever changes, we will have to add a second type
        // marker here.
        AbiParam::new(ir::types::I32).uext()
    }

    fn i64(&self) -> AbiParam {
        AbiParam::new(ir::types::I64)
    }

    fn f64(&self) -> AbiParam {
        AbiParam::new(ir::types::F64)
    }

    fn u8(&self) -> AbiParam {
        AbiParam::new(ir::types::I8)
    }

    fn wasm_signature(&self, builtin: BuiltinFunctionIndex) -> Signature {
        let mut _cur = 0;
        macro_rules! iter {
            (
                $(
                    $( #[$attr:meta] )*
                    $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
                )*
            ) => {
                $(
                    $( #[$attr] )*
                    if _cur == builtin.index() {
                        return Signature {
                            params: vec![ $( self.$param() ),* ],
                            returns: vec![ $( self.$result() )? ],
                            call_conv: self.wasm_call_conv,
                        };
                    }
                    _cur += 1;
                )*
            };
        }

        wasmtime_environ::foreach_builtin_function!(iter);

        unreachable!();
    }

    fn host_signature(&self, builtin: BuiltinFunctionIndex) -> Signature {
        let mut sig = self.wasm_signature(builtin);
        sig.call_conv = self.host_call_conv;
        sig
    }
}

/// If this bit is set on a GC reference, then the GC reference is actually an
/// unboxed `i31`.
///
/// Must be kept in sync with
/// `crate::runtime::vm::gc::VMGcRef::I31_REF_DISCRIMINANT`.
const I31_REF_DISCRIMINANT: u32 = 1;
