use cranelift_codegen::{
    MachBuffer, MachInstEmit,
    isa::aarch64,
    isa::aarch64::inst::{
        Inst,
        emit::{EmitInfo, EmitState},
    },
    settings,
};

use crate::{
    constraints::{Scope, Target},
    memory::{ReadEffect, SetEffect},
};

pub fn gpreg(i: usize) -> Target {
    let r = Target::Var("_R".to_string());
    Target::Index(Box::new(r), i)
}

pub fn vreg(i: usize) -> Target {
    let z = Target::Var("_Z".to_string());
    Target::Index(Box::new(z), i)
}

pub fn literal(s: &str) -> Target {
    Target::Var(s.to_string())
}

pub fn pstate() -> Target {
    Target::Var("PSTATE".to_string())
}

pub fn pstate_field(name: &str) -> Target {
    Target::Field(Box::new(pstate()), name.to_string())
}

pub fn fpcr() -> Target {
    Target::Var("FPCR".to_string())
}

pub fn state() -> Scope {
    let mut scope = Scope::new();

    // Boolean literals
    for lit in ["FALSE", "TRUE"] {
        scope.global(literal(lit));
    }

    // Memory effects.
    let read_effect = ReadEffect::new();
    for target in read_effect.targets() {
        scope.global(target.clone());
    }

    let set_effect = SetEffect::new();
    for target in set_effect.targets() {
        scope.global(target.clone());
    }

    // General purpose register file.
    for i in 0..31 {
        scope.global(gpreg(i));
    }

    // Vector register file.
    for i in 0..31 {
        scope.global(vreg(i));
    }

    // NZCV
    for field in &["N", "Z", "C", "V"] {
        scope.global(pstate_field(field));
    }

    // FPCR
    scope.global(fpcr());

    scope
}

/// Assemble the instruction to machine code bytes.
pub fn assemble(inst: &Inst) -> Vec<u8> {
    let flags = settings::Flags::new(settings::builder());
    let isa_flags = aarch64::settings::Flags::new(&flags, &aarch64::settings::builder());
    let emit_info = EmitInfo::new(flags, isa_flags);
    let mut buffer = MachBuffer::new();
    inst.emit(&mut buffer, &emit_info, &mut Default::default());
    let buffer = buffer.finish(&Default::default(), &mut Default::default());
    buffer.data().to_vec()
}

/// Assemble the instruction and partition into opcodes.
pub fn opcodes(inst: &Inst) -> Vec<u32> {
    let machine_code = assemble(inst);
    let mut opcodes = Vec::new();
    for opcode_bytes in machine_code.chunks(4) {
        assert_eq!(opcode_bytes.len(), 4);
        opcodes.push(u32::from_le_bytes(opcode_bytes.try_into().unwrap()));
    }
    opcodes
}

/// Assemble the instruction and returns the single opcode. Errors if the
/// instruction is not represented by a single opcode.
pub fn opcode(inst: &Inst) -> u32 {
    let opcodes = opcodes(inst);
    assert_eq!(opcodes.len(), 1);
    opcodes[0]
}

/// Assembly for the given instruction.
pub fn assembly(inst: &Inst) -> String {
    inst.print_with_state(&mut EmitState::default())
}
