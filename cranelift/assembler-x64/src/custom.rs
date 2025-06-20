pub mod encode {
    use crate::{CodeSink, KnownOffsetTable, inst};

    /// `NOP`
    pub fn nop_1b(_: &inst::nop_1b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x90);
    }

    /// `66 NOP`
    pub fn nop_2b(_: &inst::nop_2b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x66);
        buf.put1(0x90);
    }

    /// `NOP DWORD ptr [EAX]`
    pub fn nop_3b(_: &inst::nop_3b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x0F);
        buf.put1(0x1F);
        buf.put1(0x00);
    }

    /// `NOP DWORD ptr [EAX + 00H]`
    pub fn nop_4b(_: &inst::nop_4b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x0F);
        buf.put1(0x1F);
        buf.put1(0x40);
        buf.put1(0x00);
    }

    /// `NOP DWORD ptr [EAX + EAX*1 + 00H]`
    pub fn nop_5b(_: &inst::nop_5b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x0F);
        buf.put1(0x1F);
        buf.put1(0x44);
        buf.put2(0x00_00);
    }

    /// `66 NOP DWORD ptr [EAX + EAX*1 + 00H]`
    pub fn nop_6b(_: &inst::nop_6b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x66);
        buf.put1(0x0F);
        buf.put1(0x1F);
        buf.put1(0x44);
        buf.put2(0x00_00);
    }

    /// `NOP DWORD ptr [EAX + 00000000H]`
    pub fn nop_7b(_: &inst::nop_7b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x0F);
        buf.put1(0x1F);
        buf.put1(0x80);
        buf.put4(0x00_00_00_00);
    }

    /// `NOP DWORD ptr [EAX + EAX*1 + 00000000H]`
    pub fn nop_8b(_: &inst::nop_8b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x0F);
        buf.put1(0x1F);
        buf.put1(0x84);
        buf.put1(0x00);
        buf.put4(0x00_00_00_00);
    }

    /// `66 NOP DWORD ptr [EAX + EAX*1 + 00000000H]`
    pub fn nop_9b(_: &inst::nop_9b, buf: &mut impl CodeSink, _: &impl KnownOffsetTable) {
        buf.put1(0x66);
        buf.put1(0x0F);
        buf.put1(0x1F);
        buf.put1(0x84);
        buf.put1(0x00);
        buf.put4(0x00_00_00_00);
    }
}

pub mod mnemonic {
    use crate::inst;
    use crate::{Registers, XmmMem};
    use std::borrow::Cow;

    macro_rules! lock {
        ($name:tt => $mnemonic:expr) => {
            pub fn $name<R: Registers>(_: &inst::$name<R>) -> Cow<'static, str> {
                Cow::Borrowed(concat!("lock ", $mnemonic))
            }
        };
    }

    lock!(lock_addb_mi => "addb");
    lock!(lock_addw_mi => "addw");
    lock!(lock_addl_mi => "addl");
    lock!(lock_addq_mi_sxl => "addq");
    lock!(lock_addl_mi_sxb => "addl");
    lock!(lock_addq_mi_sxb => "addq");
    lock!(lock_addb_mr => "addb");
    lock!(lock_addw_mr => "addw");
    lock!(lock_addl_mr => "addl");
    lock!(lock_addq_mr => "addq");

    lock!(lock_adcb_mi => "adcb");
    lock!(lock_adcw_mi => "adcw");
    lock!(lock_adcl_mi => "adcl");
    lock!(lock_adcq_mi_sxl => "adcq");
    lock!(lock_adcl_mi_sxb => "adcl");
    lock!(lock_adcq_mi_sxb => "adcq");
    lock!(lock_adcb_mr => "adcb");
    lock!(lock_adcw_mr => "adcw");
    lock!(lock_adcl_mr => "adcl");
    lock!(lock_adcq_mr => "adcq");

    lock!(lock_subb_mi => "subb");
    lock!(lock_subw_mi => "subw");
    lock!(lock_subl_mi => "subl");
    lock!(lock_subq_mi_sxl => "subq");
    lock!(lock_subl_mi_sxb => "subl");
    lock!(lock_subq_mi_sxb => "subq");
    lock!(lock_subb_mr => "subb");
    lock!(lock_subw_mr => "subw");
    lock!(lock_subl_mr => "subl");
    lock!(lock_subq_mr => "subq");

    lock!(lock_sbbb_mi => "sbbb");
    lock!(lock_sbbw_mi => "sbbw");
    lock!(lock_sbbl_mi => "sbbl");
    lock!(lock_sbbq_mi_sxl => "sbbq");
    lock!(lock_sbbl_mi_sxb => "sbbl");
    lock!(lock_sbbq_mi_sxb => "sbbq");
    lock!(lock_sbbb_mr => "sbbb");
    lock!(lock_sbbw_mr => "sbbw");
    lock!(lock_sbbl_mr => "sbbl");
    lock!(lock_sbbq_mr => "sbbq");

    lock!(lock_andb_mi => "andb");
    lock!(lock_andw_mi => "andw");
    lock!(lock_andl_mi => "andl");
    lock!(lock_andq_mi_sxl => "andq");
    lock!(lock_andl_mi_sxb => "andl");
    lock!(lock_andq_mi_sxb => "andq");
    lock!(lock_andb_mr => "andb");
    lock!(lock_andw_mr => "andw");
    lock!(lock_andl_mr => "andl");
    lock!(lock_andq_mr => "andq");

    lock!(lock_orb_mi => "orb");
    lock!(lock_orw_mi => "orw");
    lock!(lock_orl_mi => "orl");
    lock!(lock_orq_mi_sxl => "orq");
    lock!(lock_orl_mi_sxb => "orl");
    lock!(lock_orq_mi_sxb => "orq");
    lock!(lock_orb_mr => "orb");
    lock!(lock_orw_mr => "orw");
    lock!(lock_orl_mr => "orl");
    lock!(lock_orq_mr => "orq");

    lock!(lock_xorb_mi => "xorb");
    lock!(lock_xorw_mi => "xorw");
    lock!(lock_xorl_mi => "xorl");
    lock!(lock_xorq_mi_sxl => "xorq");
    lock!(lock_xorl_mi_sxb => "xorl");
    lock!(lock_xorq_mi_sxb => "xorq");
    lock!(lock_xorb_mr => "xorb");
    lock!(lock_xorw_mr => "xorw");
    lock!(lock_xorl_mr => "xorl");
    lock!(lock_xorq_mr => "xorq");

    lock!(lock_xaddb_mr => "xaddb");
    lock!(lock_xaddw_mr => "xaddw");
    lock!(lock_xaddl_mr => "xaddl");
    lock!(lock_xaddq_mr => "xaddq");

    lock!(lock_cmpxchgb_mr => "cmpxchgb");
    lock!(lock_cmpxchgw_mr => "cmpxchgw");
    lock!(lock_cmpxchgl_mr => "cmpxchgl");
    lock!(lock_cmpxchgq_mr => "cmpxchgq");
    lock!(lock_cmpxchg16b_m => "cmpxchg16b");

    pub fn vcvtpd2ps_a<R: Registers>(inst: &inst::vcvtpd2ps_a<R>) -> Cow<'static, str> {
        match inst.xmm_m128 {
            XmmMem::Xmm(_) => "vcvtpd2ps".into(),
            XmmMem::Mem(_) => "vcvtpd2psx".into(),
        }
    }

    pub fn vcvttpd2dq_a<R: Registers>(inst: &inst::vcvttpd2dq_a<R>) -> Cow<'static, str> {
        match inst.xmm_m128 {
            XmmMem::Xmm(_) => "vcvttpd2dq".into(),
            XmmMem::Mem(_) => "vcvttpd2dqx".into(),
        }
    }
}

pub mod display {
    use crate::inst;
    use crate::{Amode, Gpr, GprMem, Registers, Size};
    use std::fmt;

    pub fn pseudo_op(imm: u8) -> &'static str {
        match imm {
            0 => "eq",
            1 => "lt",
            2 => "le",
            3 => "unord",
            4 => "neq",
            5 => "nlt",
            6 => "nle",
            7 => "ord",
            _ => panic!("not a valid immediate for pseudo op"),
        }
    }

    pub fn cmpps_a<R: Registers>(f: &mut fmt::Formatter, inst: &inst::cmpps_a<R>) -> fmt::Result {
        let xmm1 = inst.xmm1.to_string();
        let xmm_m128 = inst.xmm_m128.to_string();
        let imm8 = inst.imm8.to_string();
        if inst.imm8.value() > 7 {
            return write!(f, "{} {imm8}, {xmm_m128}, {xmm1}", inst.mnemonic());
        }
        let name = format!("cmp{}ps", pseudo_op(inst.imm8.value()));
        write!(f, "{name} {xmm_m128}, {xmm1}")
    }

    pub fn cmpss_a<R: Registers>(f: &mut fmt::Formatter, inst: &inst::cmpss_a<R>) -> fmt::Result {
        let xmm1 = inst.xmm1.to_string();
        let xmm_m32 = inst.xmm_m32.to_string();
        let imm8 = inst.imm8.to_string();
        if inst.imm8.value() > 7 {
            return write!(f, "{} {imm8}, {xmm_m32}, {xmm1}", inst.mnemonic());
        }
        let name = format!("cmp{}ss", pseudo_op(inst.imm8.value()));
        write!(f, "{name} {xmm_m32}, {xmm1}")
    }

    pub fn cmpsd_a<R: Registers>(f: &mut fmt::Formatter, inst: &inst::cmpsd_a<R>) -> fmt::Result {
        let xmm1 = inst.xmm1.to_string();
        let xmm_m64 = inst.xmm_m64.to_string();
        let imm8 = inst.imm8.to_string();
        if inst.imm8.value() > 7 {
            return write!(f, "{} {imm8}, {xmm_m64}, {xmm1}", inst.mnemonic());
        }
        let name = format!("cmp{}sd", pseudo_op(inst.imm8.value()));
        write!(f, "{name} {xmm_m64}, {xmm1}")
    }

    pub fn cmppd_a<R: Registers>(f: &mut fmt::Formatter, inst: &inst::cmppd_a<R>) -> fmt::Result {
        let xmm1 = inst.xmm1.to_string();
        let xmm_m128 = inst.xmm_m128.to_string();
        let imm8 = inst.imm8.to_string();
        if inst.imm8.value() > 7 {
            return write!(f, "{} {imm8}, {xmm_m128}, {xmm1}", inst.mnemonic());
        }
        let name = format!("cmp{}pd", pseudo_op(inst.imm8.value()));
        write!(f, "{name} {xmm_m128}, {xmm1}")
    }

    pub fn nop_1b(f: &mut fmt::Formatter, _: &inst::nop_1b) -> fmt::Result {
        write!(f, "nop")
    }

    pub fn nop_2b(f: &mut fmt::Formatter, _: &inst::nop_2b) -> fmt::Result {
        write!(f, "nop")
    }

    pub fn nop_3b(f: &mut fmt::Formatter, _: &inst::nop_3b) -> fmt::Result {
        write!(f, "nopl (%rax)")
    }

    pub fn nop_4b(f: &mut fmt::Formatter, _: &inst::nop_4b) -> fmt::Result {
        write!(f, "nopl (%rax)")
    }

    pub fn nop_5b(f: &mut fmt::Formatter, _: &inst::nop_5b) -> fmt::Result {
        write!(f, "nopl (%rax, %rax)")
    }

    pub fn nop_6b(f: &mut fmt::Formatter, _: &inst::nop_6b) -> fmt::Result {
        write!(f, "nopw (%rax, %rax)")
    }

    pub fn nop_7b(f: &mut fmt::Formatter, _: &inst::nop_7b) -> fmt::Result {
        write!(f, "nopl (%rax)")
    }

    pub fn nop_8b(f: &mut fmt::Formatter, _: &inst::nop_8b) -> fmt::Result {
        write!(f, "nopl (%rax, %rax)")
    }

    pub fn nop_9b(f: &mut fmt::Formatter, _: &inst::nop_9b) -> fmt::Result {
        write!(f, "nopw (%rax, %rax)")
    }

    pub fn xchgb_rm<R: Registers>(
        f: &mut fmt::Formatter<'_>,
        inst: &inst::xchgb_rm<R>,
    ) -> fmt::Result {
        let inst::xchgb_rm { r8, m8 } = inst;
        xchg_rm::<R>(f, r8, m8, Size::Byte)
    }

    pub fn xchgw_rm<R: Registers>(
        f: &mut fmt::Formatter<'_>,
        inst: &inst::xchgw_rm<R>,
    ) -> fmt::Result {
        let inst::xchgw_rm { r16, m16 } = inst;
        xchg_rm::<R>(f, r16, m16, Size::Word)
    }

    pub fn xchgl_rm<R: Registers>(
        f: &mut fmt::Formatter<'_>,
        inst: &inst::xchgl_rm<R>,
    ) -> fmt::Result {
        let inst::xchgl_rm { r32, m32 } = inst;
        xchg_rm::<R>(f, r32, m32, Size::Doubleword)
    }

    pub fn xchgq_rm<R: Registers>(
        f: &mut fmt::Formatter<'_>,
        inst: &inst::xchgq_rm<R>,
    ) -> fmt::Result {
        let inst::xchgq_rm { r64, m64 } = inst;
        xchg_rm::<R>(f, r64, m64, Size::Quadword)
    }

    /// Swap the order of printing (register first) to match Capstone.
    fn xchg_rm<R: Registers>(
        f: &mut fmt::Formatter<'_>,
        reg: &Gpr<R::ReadWriteGpr>,
        mem: &Amode<R::ReadGpr>,
        size: Size,
    ) -> fmt::Result {
        let reg = reg.to_string(size);
        let mem = mem.to_string();
        let suffix = match size {
            Size::Byte => "b",
            Size::Word => "w",
            Size::Doubleword => "l",
            Size::Quadword => "q",
        };
        write!(f, "xchg{suffix} {reg}, {mem}")
    }

    pub fn sarb_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::sarb_m1<R>) -> fmt::Result {
        let inst::sarb_m1 { rm8 } = inst;
        shift_m1::<R>(f, "sarb", rm8, Size::Byte)
    }

    pub fn sarw_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::sarw_m1<R>) -> fmt::Result {
        let inst::sarw_m1 { rm16 } = inst;
        shift_m1::<R>(f, "sarw", rm16, Size::Word)
    }

    pub fn sarl_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::sarl_m1<R>) -> fmt::Result {
        let inst::sarl_m1 { rm32 } = inst;
        shift_m1::<R>(f, "sarl", rm32, Size::Doubleword)
    }

    pub fn sarq_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::sarq_m1<R>) -> fmt::Result {
        let inst::sarq_m1 { rm64 } = inst;
        shift_m1::<R>(f, "sarq", rm64, Size::Quadword)
    }

    pub fn shlb_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::shlb_m1<R>) -> fmt::Result {
        let inst::shlb_m1 { rm8 } = inst;
        shift_m1::<R>(f, "shlb", rm8, Size::Byte)
    }

    pub fn shlw_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::shlw_m1<R>) -> fmt::Result {
        let inst::shlw_m1 { rm16 } = inst;
        shift_m1::<R>(f, "shlw", rm16, Size::Word)
    }

    pub fn shll_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::shll_m1<R>) -> fmt::Result {
        let inst::shll_m1 { rm32 } = inst;
        shift_m1::<R>(f, "shll", rm32, Size::Doubleword)
    }

    pub fn shlq_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::shlq_m1<R>) -> fmt::Result {
        let inst::shlq_m1 { rm64 } = inst;
        shift_m1::<R>(f, "shlq", rm64, Size::Quadword)
    }

    pub fn shrb_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::shrb_m1<R>) -> fmt::Result {
        let inst::shrb_m1 { rm8 } = inst;
        shift_m1::<R>(f, "shrb", rm8, Size::Byte)
    }

    pub fn shrw_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::shrw_m1<R>) -> fmt::Result {
        let inst::shrw_m1 { rm16 } = inst;
        shift_m1::<R>(f, "shrw", rm16, Size::Word)
    }

    pub fn shrl_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::shrl_m1<R>) -> fmt::Result {
        let inst::shrl_m1 { rm32 } = inst;
        shift_m1::<R>(f, "shrl", rm32, Size::Doubleword)
    }

    pub fn shrq_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::shrq_m1<R>) -> fmt::Result {
        let inst::shrq_m1 { rm64 } = inst;
        shift_m1::<R>(f, "shrq", rm64, Size::Quadword)
    }

    pub fn rorb_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::rorb_m1<R>) -> fmt::Result {
        let inst::rorb_m1 { rm8 } = inst;
        shift_m1::<R>(f, "rorb", rm8, Size::Byte)
    }

    pub fn rorw_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::rorw_m1<R>) -> fmt::Result {
        let inst::rorw_m1 { rm16 } = inst;
        shift_m1::<R>(f, "rorw", rm16, Size::Word)
    }

    pub fn rorl_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::rorl_m1<R>) -> fmt::Result {
        let inst::rorl_m1 { rm32 } = inst;
        shift_m1::<R>(f, "rorl", rm32, Size::Doubleword)
    }

    pub fn rorq_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::rorq_m1<R>) -> fmt::Result {
        let inst::rorq_m1 { rm64 } = inst;
        shift_m1::<R>(f, "rorq", rm64, Size::Quadword)
    }

    pub fn rolb_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::rolb_m1<R>) -> fmt::Result {
        let inst::rolb_m1 { rm8 } = inst;
        shift_m1::<R>(f, "rolb", rm8, Size::Byte)
    }

    pub fn rolw_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::rolw_m1<R>) -> fmt::Result {
        let inst::rolw_m1 { rm16 } = inst;
        shift_m1::<R>(f, "rolw", rm16, Size::Word)
    }

    pub fn roll_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::roll_m1<R>) -> fmt::Result {
        let inst::roll_m1 { rm32 } = inst;
        shift_m1::<R>(f, "roll", rm32, Size::Doubleword)
    }

    pub fn rolq_m1<R: Registers>(f: &mut fmt::Formatter, inst: &inst::rolq_m1<R>) -> fmt::Result {
        let inst::rolq_m1 { rm64 } = inst;
        shift_m1::<R>(f, "rolq", rm64, Size::Quadword)
    }

    fn shift_m1<R: Registers>(
        f: &mut fmt::Formatter<'_>,
        mnemonic: &str,
        rm: &GprMem<R::ReadWriteGpr, R::ReadGpr>,
        size: Size,
    ) -> fmt::Result {
        let reg = rm.to_string(size);
        match rm {
            GprMem::Gpr(_) => write!(f, "{mnemonic} $1, {reg}"),
            GprMem::Mem(_) => write!(f, "{mnemonic} {reg}"),
        }
    }
}

pub mod visit {
    use crate::inst::*;
    use crate::{Amode, Fixed, Gpr, GprMem, RegisterVisitor, Registers, gpr};

    pub fn mulxl_rvm<R: Registers>(mulx: &mut mulxl_rvm<R>, visitor: &mut impl RegisterVisitor<R>) {
        visit_mulx(
            &mut mulx.r32a,
            &mut mulx.r32b,
            &mut mulx.rm32,
            &mut mulx.edx,
            visitor,
        )
    }

    pub fn mulxq_rvm<R: Registers>(mulx: &mut mulxq_rvm<R>, visitor: &mut impl RegisterVisitor<R>) {
        visit_mulx(
            &mut mulx.r64a,
            &mut mulx.r64b,
            &mut mulx.rm64,
            &mut mulx.rdx,
            visitor,
        )
    }

    /// Both mulxl and mulxq have custom register allocator behavior where if the
    /// two writable registers are the same then only one is flagged as writable.
    /// That represents how when they're both the same only one register is written,
    /// not both.
    fn visit_mulx<R: Registers>(
        ra: &mut Gpr<R::WriteGpr>,
        rb: &mut Gpr<R::WriteGpr>,
        src1: &mut GprMem<R::ReadGpr, R::ReadGpr>,
        src2: &mut Fixed<R::ReadGpr, { gpr::enc::RDX }>,
        visitor: &mut impl RegisterVisitor<R>,
    ) {
        if ra == rb {
            visitor.write_gpr(ra.as_mut());
            *rb = *ra;
        } else {
            visitor.write_gpr(ra.as_mut());
            visitor.write_gpr(rb.as_mut());
        }
        visitor.read_gpr_mem(src1);
        let enc = src2.expected_enc();
        visitor.fixed_read_gpr(&mut src2.0, enc);
    }

    pub fn lock_xaddb_mr<R: Registers>(
        lock_xadd: &mut lock_xaddb_mr<R>,
        visitor: &mut impl RegisterVisitor<R>,
    ) {
        let lock_xaddb_mr { r8, m8 } = lock_xadd;
        lock_xadd_mr(r8, m8, visitor)
    }

    pub fn lock_xaddw_mr<R: Registers>(
        lock_xadd: &mut lock_xaddw_mr<R>,
        visitor: &mut impl RegisterVisitor<R>,
    ) {
        let lock_xaddw_mr { r16, m16 } = lock_xadd;
        lock_xadd_mr(r16, m16, visitor)
    }

    pub fn lock_xaddl_mr<R: Registers>(
        lock_xadd: &mut lock_xaddl_mr<R>,
        visitor: &mut impl RegisterVisitor<R>,
    ) {
        let lock_xaddl_mr { r32, m32 } = lock_xadd;
        lock_xadd_mr(r32, m32, visitor)
    }

    pub fn lock_xaddq_mr<R: Registers>(
        lock_xadd: &mut lock_xaddq_mr<R>,
        visitor: &mut impl RegisterVisitor<R>,
    ) {
        let lock_xaddq_mr { r64, m64 } = lock_xadd;
        lock_xadd_mr(r64, m64, visitor)
    }

    /// Intel says the memory operand comes first, but regalloc requires the
    /// register operand comes first, so the custom visit implementation here
    /// resolves that.
    fn lock_xadd_mr<R: Registers>(
        reg: &mut Gpr<R::ReadWriteGpr>,
        mem: &mut Amode<R::ReadGpr>,
        visitor: &mut impl RegisterVisitor<R>,
    ) {
        visitor.read_write_gpr(reg.as_mut());
        visitor.read_amode(mem);
    }
}
