//! This crate defines `TestOperator`: a `TOperator` type for usage in tests.
//!
//! This allows us to write Peepmatic-specific tests that do not depend on
//! building all of Cranelift.

peepmatic_traits::define_operator! {
    /// A `TOperator` type for use inside tests.
    TestOperator {
        adjust_sp_down => AdjustSpDown {
            parameters(iNN);
            result(void);
        }
        adjust_sp_down_imm => AdjustSpDownImm {
            immediates(iNN);
            result(void);
        }
        band => Band {
            parameters(iNN, iNN);
            result(iNN);
        }
        band_imm => BandImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        bconst => Bconst {
            immediates(b1);
            result(bNN);
        }
        bint => Bint {
            parameters(bNN);
            result(iNN);
        }
        bor => Bor {
            parameters(iNN, iNN);
            result(iNN);
        }
        bor_imm => BorImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        brnz => Brnz {
            parameters(bool_or_int);
            result(void);
        }
        brz => Brz {
            parameters(bool_or_int);
            result(void);
        }
        bxor => Bxor {
            parameters(iNN, iNN);
            result(iNN);
        }
        bxor_imm => BxorImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        iadd => Iadd {
            parameters(iNN, iNN);
            result(iNN);
        }
        iadd_imm => IaddImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        icmp => Icmp {
            immediates(cc);
            parameters(iNN, iNN);
            result(b1);
        }
        icmp_imm => IcmpImm {
            immediates(cc, iNN);
            parameters(iNN);
            result(b1);
        }
        iconst => Iconst {
            immediates(iNN);
            result(iNN);
        }
        ifcmp => Ifcmp {
            parameters(iNN, iNN);
            result(cpu_flags);
        }
        ifcmp_imm => IfcmpImm {
            immediates(iNN);
            parameters(iNN);
            result(cpu_flags);
        }
        imul => Imul {
            parameters(iNN, iNN);
            result(iNN);
        }
        imul_imm => ImulImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        ireduce => Ireduce {
            parameters(iNN);
            result(iMM);
            is_reduce(true);
        }
        irsub_imm => IrsubImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        ishl => Ishl {
            parameters(iNN, iNN);
            result(iNN);
        }
        ishl_imm => IshlImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        isub => Isub {
            parameters(iNN, iNN);
            result(iNN);
        }
        rotl => Rotl {
            parameters(iNN, iNN);
            result(iNN);
        }
        rotl_imm => RotlImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        rotr => Rotr {
            parameters(iNN, iNN);
            result(iNN);
        }
        rotr_imm => RotrImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        sdiv => Sdiv {
            parameters(iNN, iNN);
            result(iNN);
        }
        sdiv_imm => SdivImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        select => Select {
            parameters(bool_or_int, any_t, any_t);
            result(any_t);
        }
        sextend => Sextend {
            parameters(iNN);
            result(iMM);
            is_extend(true);
        }
        srem => Srem {
            parameters(iNN, iNN);
            result(iNN);
        }
        srem_imm => SremImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        sshr => Sshr {
            parameters(iNN, iNN);
            result(iNN);
        }
        sshr_imm => SshrImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        trapnz => Trapnz {
            parameters(bool_or_int);
            result(void);
        }
        trapz => Trapz {
            parameters(bool_or_int);
            result(void);
        }
        udiv => Udiv {
            parameters(iNN, iNN);
            result(iNN);
        }
        udiv_imm => UdivImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        uextend => Uextend {
            parameters(iNN);
            result(iMM);
            is_extend(true);
        }
        urem => Urem {
            parameters(iNN, iNN);
            result(iNN);
        }
        urem_imm => UremImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        ushr => Ushr {
            parameters(iNN, iNN);
            result(iNN);
        }
        ushr_imm => UshrImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
    }
}
