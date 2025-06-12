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
    use crate::Registers;
    use crate::inst;

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
            _ => panic!("not a u8"),
        }
    }

    pub fn cmpps_a<R: Registers>(
        f: &mut std::fmt::Formatter,
        inst: &inst::cmpps_a<R>,
    ) -> std::fmt::Result {
        let xmm1 = inst.xmm1.to_string();
        let xmm_m128 = inst.xmm_m128.to_string();
        let imm8 = inst.imm8.to_string();
        if inst.imm8.value() > 7 {
            return write!(f, "{} {imm8}, {xmm_m128}, {xmm1}", inst.mnemonic());
        }
        let name = format!("cmp{}ps", pseudo_op(inst.imm8.value()));
        write!(f, "{name} {xmm_m128}, {xmm1}")
    }

    pub fn cmpss_a<R: Registers>(
        f: &mut std::fmt::Formatter,
        inst: &inst::cmpss_a<R>,
    ) -> std::fmt::Result {
        let xmm1 = inst.xmm1.to_string();
        let xmm_m32 = inst.xmm_m32.to_string();
        let imm8 = inst.imm8.to_string();
        if inst.imm8.value() > 7 {
            return write!(f, "{} {imm8}, {xmm_m32}, {xmm1}", inst.mnemonic());
        }
        let name = format!("cmp{}ss", pseudo_op(inst.imm8.value()));
        write!(f, "{name} {xmm_m32}, {xmm1}")
    }

    pub fn cmpsd_a<R: Registers>(
        f: &mut std::fmt::Formatter,
        inst: &inst::cmpsd_a<R>,
    ) -> std::fmt::Result {
        let xmm1 = inst.xmm1.to_string();
        let xmm_m64 = inst.xmm_m64.to_string();
        let imm8 = inst.imm8.to_string();
        if inst.imm8.value() > 7 {
            return write!(f, "{} {imm8}, {xmm_m64}, {xmm1}", inst.mnemonic());
        }
        let name = format!("cmp{}sd", pseudo_op(inst.imm8.value()));
        write!(f, "{name} {xmm_m64}, {xmm1}")
    }

    pub fn cmppd_a<R: Registers>(
        f: &mut std::fmt::Formatter,
        inst: &inst::cmppd_a<R>,
    ) -> std::fmt::Result {
        let xmm1 = inst.xmm1.to_string();
        let xmm_m128 = inst.xmm_m128.to_string();
        let imm8 = inst.imm8.to_string();
        if inst.imm8.value() > 7 {
            return write!(f, "{} {imm8}, {xmm_m128}, {xmm1}", inst.mnemonic());
        }
        let name = format!("cmp{}pd", pseudo_op(inst.imm8.value()));
        write!(f, "{name} {xmm_m128}, {xmm1}")
    }
}

pub mod visit {
    use crate::inst::{mulxl_rvm, mulxq_rvm};
    use crate::{Fixed, Gpr, GprMem, RegisterVisitor, Registers, gpr};

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
}
