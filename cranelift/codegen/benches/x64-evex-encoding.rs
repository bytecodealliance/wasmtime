//! Measure instruction encoding latency using various approaches; the
//! benchmarking is feature-gated on `x86` since it only measures the encoding
//! mechanism of that backend.

#[cfg(feature = "x86")]
mod x86 {
    use cranelift_codegen::isa::x64::encoding::{
        evex::{EvexContext, EvexInstruction, EvexMasking, EvexVectorLength, Register},
        rex::OpcodeMap,
        rex::{encode_modrm, LegacyPrefixes},
        ByteSink,
    };
    use cranelift_codegen_shared::isa::x86::EncodingBits;
    use criterion::{criterion_group, Criterion};

    // Define the benchmarks.
    fn x64_evex_encoding_benchmarks(c: &mut Criterion) {
        let mut group = c.benchmark_group("x64 EVEX encoding");
        let rax = Register::from(0);
        let rdx = Register::from(2);

        group.bench_function("EvexInstruction (builder pattern)", |b| {
            let mut sink = vec![];
            b.iter(|| {
                sink.clear();
                EvexInstruction::new()
                    .prefix(LegacyPrefixes::_66)
                    .map(OpcodeMap::_0F38)
                    .w(true)
                    .opcode(0x1F)
                    .reg(rax)
                    .rm(rdx)
                    .length(EvexVectorLength::V128)
                    .encode(&mut sink);
            });
        });

        group.bench_function("encode_evex (function pattern)", |b| {
            let mut sink = vec![];
            let bits = EncodingBits::new(&[0x66, 0x0f, 0x38, 0x1f], 0, 1);
            let vvvvv = Register::from(0);
            b.iter(|| {
                sink.clear();
                encode_evex(
                    bits,
                    rax,
                    vvvvv,
                    rdx,
                    EvexContext::Other {
                        length: EvexVectorLength::V128,
                    },
                    EvexMasking::default(),
                    &mut sink,
                );
            })
        });
    }
    criterion_group!(benches, x64_evex_encoding_benchmarks);

    /// Using an inner module to feature-gate the benchmarks means that we must
    /// manually specify how to run the benchmarks (see `criterion_main!`).
    pub fn run_benchmarks() {
        criterion::__warn_about_html_reports_feature();
        criterion::__warn_about_cargo_bench_support_feature();
        benches();
        Criterion::default().configure_from_args().final_summary();
    }

    /// From the legacy x86 backend: a mechanism for encoding an EVEX
    /// instruction, including the prefixes, the instruction opcode, and the
    /// ModRM byte. This EVEX encoding function only encodes the `reg` (operand
    /// 1), `vvvv` (operand 2), `rm` (operand 3) form; other forms are possible
    /// (see section 2.6.2, Intel Software Development Manual, volume 2A),
    /// requiring refactoring of this function or separate functions for each
    /// form (e.g. as for the REX prefix).
    #[inline(always)]
    pub fn encode_evex<CS: ByteSink + ?Sized>(
        enc: EncodingBits,
        reg: Register,
        vvvvv: Register,
        rm: Register,
        context: EvexContext,
        masking: EvexMasking,
        sink: &mut CS,
    ) {
        let reg: u8 = reg.into();
        let rm: u8 = rm.into();
        let vvvvv: u8 = vvvvv.into();

        // EVEX prefix.
        sink.put1(0x62);

        debug_assert!(enc.mm() < 0b100);
        let mut p0 = enc.mm() & 0b11;
        p0 |= evex2(rm, reg) << 4; // bits 3:2 are always unset
        sink.put1(p0);

        let mut p1 = enc.pp() | 0b100; // bit 2 is always set
        p1 |= (!(vvvvv) & 0b1111) << 3;
        p1 |= (enc.rex_w() & 0b1) << 7;
        sink.put1(p1);

        let mut p2 = masking.aaa_bits();
        p2 |= (!(vvvvv >> 4) & 0b1) << 3;
        p2 |= context.bits() << 4;
        p2 |= masking.z_bit() << 7;
        sink.put1(p2);

        // Opcode.
        sink.put1(enc.opcode_byte());

        // ModR/M byte.
        sink.put1(encode_modrm(3, reg & 7, rm & 7))
    }

    /// From the legacy x86 backend: encode the RXBR' bits of the EVEX P0 byte.
    /// For an explanation of these bits, see section 2.6.1 in the Intel
    /// Software Development Manual, volume 2A. These bits can be used by
    /// different addressing modes (see section 2.6.2), requiring different
    /// `vex*` functions than this one.
    fn evex2(rm: u8, reg: u8) -> u8 {
        let b = !(rm >> 3) & 1;
        let x = !(rm >> 4) & 1;
        let r = !(reg >> 3) & 1;
        let r_ = !(reg >> 4) & 1;
        0x00 | r_ | (b << 1) | (x << 2) | (r << 3)
    }
}

fn main() {
    #[cfg(feature = "x86")]
    x86::run_benchmarks();

    #[cfg(not(feature = "x86"))]
    println!(
        "Unable to run the x64-evex-encoding benchmark; the `x86` feature must be enabled in Cargo.",
    );
}
