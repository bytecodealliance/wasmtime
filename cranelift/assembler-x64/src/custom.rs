use crate::inst::{mulxl_rvm, mulxq_rvm};
use crate::{Fixed, Gpr, GprMem, RegisterVisitor, Registers, gpr};

pub fn visit_mulxl_rvm<R: Registers>(
    mulx: &mut mulxl_rvm<R>,
    visitor: &mut impl RegisterVisitor<R>,
) {
    visit_mulx(
        &mut mulx.r32a,
        &mut mulx.r32b,
        &mut mulx.rm32,
        &mut mulx.edx,
        visitor,
    )
}

pub fn visit_mulxq_rvm<R: Registers>(
    mulx: &mut mulxq_rvm<R>,
    visitor: &mut impl RegisterVisitor<R>,
) {
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
