#![allow(dead_code)] // for now

use dynasmrt::x64::Assembler;
use dynasmrt::DynasmApi;

type GPR = u8;

struct GPRs {
    bits: u16,
}

impl GPRs {
    fn new() -> Self {
        Self { bits: 0 }
    }
}

static RAX: u8 = 0;
static RCX: u8 = 1;
static RDX: u8 = 2;
static RBX: u8 = 3;
static RSP: u8 = 4;
static RBP: u8 = 5;
static RSI: u8 = 6;
static RDI: u8 = 7;
static R8: u8 = 8;
static R9: u8 = 9;
static R10: u8 = 10;
static R11: u8 = 11;
static R12: u8 = 12;
static R13: u8 = 13;
static R14: u8 = 14;
static R15: u8 = 15;

impl GPRs {
    fn take(&mut self) -> GPR {
        let lz = self.bits.trailing_zeros();
        assert!(lz < 32, "ran out of free GPRs");
        self.bits &= !(1 << lz);
        lz as GPR
    }

    fn release(&mut self, gpr: GPR) {
        assert_eq!(
            self.bits & (1 << gpr),
            0,
            "released register was already free"
        );
        self.bits |= 1 << gpr;
    }
}

pub struct Registers {
    scratch_gprs: GPRs,
}

impl Registers {
    pub fn new() -> Self {
        let mut result = Self {
            scratch_gprs: GPRs::new(),
        };
        // Give ourselves a few scratch registers to work with, for now.
        result.release_scratch_gpr(RAX);
        result.release_scratch_gpr(RCX);
        result.release_scratch_gpr(RDX);
        result
    }

    pub fn take_scratch_gpr(&mut self) -> GPR {
        self.scratch_gprs.take()
    }

    pub fn release_scratch_gpr(&mut self, gpr: GPR) {
        self.scratch_gprs.release(gpr);
    }
}

fn push_i32(ops: &mut Assembler, regs: &mut Registers, gpr: GPR) {
    // For now, do an actual push (and pop below). In the future, we could
    // do on-the-fly register allocation here.
    dynasm!(ops
        ; push Rq(gpr)
    );
    regs.release_scratch_gpr(gpr);
}

fn pop_i32(ops: &mut Assembler, regs: &mut Registers) -> GPR {
    let gpr = regs.take_scratch_gpr();
    dynasm!(ops
        ; pop Rq(gpr)
    );
    gpr
}

pub fn add_i32(ops: &mut Assembler, regs: &mut Registers) {
    let op0 = pop_i32(ops, regs);
    let op1 = pop_i32(ops, regs);
    dynasm!(ops
        ; add Rq(op0), Rq(op1)
    );
    push_i32(ops, regs, op0);
    regs.release_scratch_gpr(op1);
}

pub fn unsupported_opcode(ops: &mut Assembler) {
    dynasm!(ops
        ; ud2
    );
}
