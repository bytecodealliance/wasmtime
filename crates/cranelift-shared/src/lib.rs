use cranelift_codegen::{
    binemit,
    ir::{self, ExternalName, UserExternalNameRef},
    settings, FinalizedMachReloc, FinalizedRelocTarget, MachTrap,
};
use wasmtime_environ::{FlagValue, FuncIndex, Trap, TrapInformation};

pub mod isa_builder;
mod obj;
pub use obj::*;
mod compiled_function;
pub use compiled_function::*;

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

/// Destination function. Can be either user function or some special one, like `memory.grow`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RelocationTarget {
    /// The user function index.
    UserFunc(FuncIndex),
    /// A compiler-generated libcall.
    LibCall(ir::LibCall),
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

/// Trap code used for debug assertions we emit in our JIT code.
const DEBUG_ASSERT_TRAP_CODE: u16 = u16::MAX;

/// A custom code with `TrapCode::User` which is used by always-trap shims which
/// indicates that, as expected, the always-trapping function indeed did trap.
/// This effectively provides a better error message as opposed to a bland
/// "unreachable code reached"
pub const ALWAYS_TRAP_CODE: u16 = 100;

/// A custom code with `TrapCode::User` corresponding to being unable to reenter
/// a component due to its reentrance limitations. This is used in component
/// adapters to provide a more useful error message in such situations.
pub const CANNOT_ENTER_CODE: u16 = 101;

/// Converts machine traps to trap information.
pub fn mach_trap_to_trap(trap: &MachTrap) -> Option<TrapInformation> {
    let &MachTrap { offset, code } = trap;
    Some(TrapInformation {
        code_offset: offset,
        trap_code: match code {
            ir::TrapCode::StackOverflow => Trap::StackOverflow,
            ir::TrapCode::HeapOutOfBounds => Trap::MemoryOutOfBounds,
            ir::TrapCode::HeapMisaligned => Trap::HeapMisaligned,
            ir::TrapCode::TableOutOfBounds => Trap::TableOutOfBounds,
            ir::TrapCode::IndirectCallToNull => Trap::IndirectCallToNull,
            ir::TrapCode::BadSignature => Trap::BadSignature,
            ir::TrapCode::IntegerOverflow => Trap::IntegerOverflow,
            ir::TrapCode::IntegerDivisionByZero => Trap::IntegerDivisionByZero,
            ir::TrapCode::BadConversionToInteger => Trap::BadConversionToInteger,
            ir::TrapCode::UnreachableCodeReached => Trap::UnreachableCodeReached,
            ir::TrapCode::Interrupt => Trap::Interrupt,
            ir::TrapCode::User(ALWAYS_TRAP_CODE) => Trap::AlwaysTrapAdapter,
            ir::TrapCode::User(CANNOT_ENTER_CODE) => Trap::CannotEnterComponent,
            ir::TrapCode::NullReference => Trap::NullReference,
            ir::TrapCode::NullI31Ref => Trap::NullI31Ref,

            // These do not get converted to wasmtime traps, since they
            // shouldn't ever be hit in theory. Instead of catching and handling
            // these, we let the signal crash the process.
            ir::TrapCode::User(DEBUG_ASSERT_TRAP_CODE) => return None,

            // these should never be emitted by wasmtime-cranelift
            ir::TrapCode::User(_) => unreachable!(),
        },
    })
}

/// Converts machine relocations to relocation information
/// to perform.
fn mach_reloc_to_reloc<F>(reloc: &FinalizedMachReloc, transform_user_func_ref: F) -> Relocation
where
    F: Fn(UserExternalNameRef) -> (u32, u32),
{
    let &FinalizedMachReloc {
        offset,
        kind,
        ref target,
        addend,
    } = reloc;
    let reloc_target = match *target {
        FinalizedRelocTarget::ExternalName(ExternalName::User(user_func_ref)) => {
            let (namespace, index) = transform_user_func_ref(user_func_ref);
            debug_assert_eq!(namespace, 0);
            RelocationTarget::UserFunc(FuncIndex::from_u32(index))
        }
        FinalizedRelocTarget::ExternalName(ExternalName::LibCall(libcall)) => {
            RelocationTarget::LibCall(libcall)
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
