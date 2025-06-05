pub mod display {
    macro_rules! lock {
        ($name:tt) => {
            pub fn $name<R: $crate::Registers>(inst: &$crate::inst::$name<R>) -> String {
                format!("lock {}", &inst.mnemonic()[5..])
            }
        };
    }

    lock!(lock_addb_mi);
    lock!(lock_addw_mi);
    lock!(lock_addl_mi);
    lock!(lock_addq_mi_sxl);
    lock!(lock_addl_mi_sxb);
    lock!(lock_addq_mi_sxb);
    lock!(lock_addb_mr);
    lock!(lock_addw_mr);
    lock!(lock_addl_mr);
    lock!(lock_addq_mr);

    lock!(lock_adcb_mi);
    lock!(lock_adcw_mi);
    lock!(lock_adcl_mi);
    lock!(lock_adcq_mi_sxl);
    lock!(lock_adcl_mi_sxb);
    lock!(lock_adcq_mi_sxb);
    lock!(lock_adcb_mr);
    lock!(lock_adcw_mr);
    lock!(lock_adcl_mr);
    lock!(lock_adcq_mr);

    lock!(lock_subb_mi);
    lock!(lock_subw_mi);
    lock!(lock_subl_mi);
    lock!(lock_subq_mi_sxl);
    lock!(lock_subl_mi_sxb);
    lock!(lock_subq_mi_sxb);
    lock!(lock_subb_mr);
    lock!(lock_subw_mr);
    lock!(lock_subl_mr);
    lock!(lock_subq_mr);

    lock!(lock_sbbb_mi);
    lock!(lock_sbbw_mi);
    lock!(lock_sbbl_mi);
    lock!(lock_sbbq_mi_sxl);
    lock!(lock_sbbl_mi_sxb);
    lock!(lock_sbbq_mi_sxb);
    lock!(lock_sbbb_mr);
    lock!(lock_sbbw_mr);
    lock!(lock_sbbl_mr);
    lock!(lock_sbbq_mr);

    lock!(lock_andb_mi);
    lock!(lock_andw_mi);
    lock!(lock_andl_mi);
    lock!(lock_andq_mi_sxl);
    lock!(lock_andl_mi_sxb);
    lock!(lock_andq_mi_sxb);
    lock!(lock_andb_mr);
    lock!(lock_andw_mr);
    lock!(lock_andl_mr);
    lock!(lock_andq_mr);

    lock!(lock_orb_mi);
    lock!(lock_orw_mi);
    lock!(lock_orl_mi);
    lock!(lock_orq_mi_sxl);
    lock!(lock_orl_mi_sxb);
    lock!(lock_orq_mi_sxb);
    lock!(lock_orb_mr);
    lock!(lock_orw_mr);
    lock!(lock_orl_mr);
    lock!(lock_orq_mr);

    lock!(lock_xorb_mi);
    lock!(lock_xorw_mi);
    lock!(lock_xorl_mi);
    lock!(lock_xorq_mi_sxl);
    lock!(lock_xorl_mi_sxb);
    lock!(lock_xorq_mi_sxb);
    lock!(lock_xorb_mr);
    lock!(lock_xorw_mr);
    lock!(lock_xorl_mr);
    lock!(lock_xorq_mr);
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
