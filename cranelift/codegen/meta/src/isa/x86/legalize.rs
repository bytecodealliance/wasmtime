use crate::cdsl::ast::{constant, var, ExprBuilder, Literal};
use crate::cdsl::instructions::{vector, Bindable, InstructionGroup};
use crate::cdsl::types::{LaneType, ValueType};
use crate::cdsl::xform::TransformGroupBuilder;
use crate::shared::types::Float::{F32, F64};
use crate::shared::types::Int::{I16, I32, I64, I8};
use crate::shared::Definitions as SharedDefinitions;

#[allow(clippy::many_single_char_names)]
pub(crate) fn define(shared: &mut SharedDefinitions, x86_instructions: &InstructionGroup) {
    let mut group = TransformGroupBuilder::new(
        "x86_expand",
        r#"
    Legalize instructions by expansion.

    Use x86-specific instructions if needed."#,
    )
    .isa("x86")
    .chain_with(shared.transform_groups.by_name("expand_flags").id);

    // List of instructions.
    let insts = &shared.instructions;
    let band = insts.by_name("band");
    let band_not = insts.by_name("band_not");
    let bitcast = insts.by_name("bitcast");
    let bitselect = insts.by_name("bitselect");
    let bor = insts.by_name("bor");
    let bnot = insts.by_name("bnot");
    let bxor = insts.by_name("bxor");
    let clz = insts.by_name("clz");
    let ctz = insts.by_name("ctz");
    let extractlane = insts.by_name("extractlane");
    let fcmp = insts.by_name("fcmp");
    let fcvt_from_uint = insts.by_name("fcvt_from_uint");
    let fcvt_to_sint = insts.by_name("fcvt_to_sint");
    let fcvt_to_uint = insts.by_name("fcvt_to_uint");
    let fcvt_to_sint_sat = insts.by_name("fcvt_to_sint_sat");
    let fcvt_to_uint_sat = insts.by_name("fcvt_to_uint_sat");
    let fabs = insts.by_name("fabs");
    let fmax = insts.by_name("fmax");
    let fmin = insts.by_name("fmin");
    let fneg = insts.by_name("fneg");
    let iadd = insts.by_name("iadd");
    let icmp = insts.by_name("icmp");
    let iconst = insts.by_name("iconst");
    let imul = insts.by_name("imul");
    let ineg = insts.by_name("ineg");
    let insertlane = insts.by_name("insertlane");
    let ishl = insts.by_name("ishl");
    let ishl_imm = insts.by_name("ishl_imm");
    let isub = insts.by_name("isub");
    let popcnt = insts.by_name("popcnt");
    let raw_bitcast = insts.by_name("raw_bitcast");
    let scalar_to_vector = insts.by_name("scalar_to_vector");
    let sdiv = insts.by_name("sdiv");
    let selectif = insts.by_name("selectif");
    let smulhi = insts.by_name("smulhi");
    let splat = insts.by_name("splat");
    let shuffle = insts.by_name("shuffle");
    let srem = insts.by_name("srem");
    let sshr = insts.by_name("sshr");
    let trueif = insts.by_name("trueif");
    let udiv = insts.by_name("udiv");
    let umulhi = insts.by_name("umulhi");
    let ushr_imm = insts.by_name("ushr_imm");
    let urem = insts.by_name("urem");
    let ushr = insts.by_name("ushr");
    let vconst = insts.by_name("vconst");
    let vall_true = insts.by_name("vall_true");
    let vany_true = insts.by_name("vany_true");

    let x86_bsf = x86_instructions.by_name("x86_bsf");
    let x86_bsr = x86_instructions.by_name("x86_bsr");
    let x86_pmaxu = x86_instructions.by_name("x86_pmaxu");
    let x86_pmins = x86_instructions.by_name("x86_pmins");
    let x86_pminu = x86_instructions.by_name("x86_pminu");
    let x86_pshufb = x86_instructions.by_name("x86_pshufb");
    let x86_pshufd = x86_instructions.by_name("x86_pshufd");
    let x86_psll = x86_instructions.by_name("x86_psll");
    let x86_psra = x86_instructions.by_name("x86_psra");
    let x86_psrl = x86_instructions.by_name("x86_psrl");
    let x86_ptest = x86_instructions.by_name("x86_ptest");
    let x86_umulx = x86_instructions.by_name("x86_umulx");
    let x86_smulx = x86_instructions.by_name("x86_smulx");

    let imm = &shared.imm;

    // Division and remainder.
    //
    // The srem expansion requires custom code because srem INT_MIN, -1 is not
    // allowed to trap. The other ops need to check avoid_div_traps.
    group.custom_legalize(sdiv, "expand_sdivrem");
    group.custom_legalize(srem, "expand_sdivrem");
    group.custom_legalize(udiv, "expand_udivrem");
    group.custom_legalize(urem, "expand_udivrem");

    // Double length (widening) multiplication.
    let a = var("a");
    let x = var("x");
    let y = var("y");
    let a1 = var("a1");
    let a2 = var("a2");
    let res_lo = var("res_lo");
    let res_hi = var("res_hi");

    group.legalize(
        def!(res_hi = umulhi(x, y)),
        vec![def!((res_lo, res_hi) = x86_umulx(x, y))],
    );

    group.legalize(
        def!(res_hi = smulhi(x, y)),
        vec![def!((res_lo, res_hi) = x86_smulx(x, y))],
    );

    // Floating point condition codes.
    //
    // The 8 condition codes in `supported_floatccs` are directly supported by a
    // `ucomiss` or `ucomisd` instruction. The remaining codes need legalization
    // patterns.

    let floatcc_eq = Literal::enumerator_for(&imm.floatcc, "eq");
    let floatcc_ord = Literal::enumerator_for(&imm.floatcc, "ord");
    let floatcc_ueq = Literal::enumerator_for(&imm.floatcc, "ueq");
    let floatcc_ne = Literal::enumerator_for(&imm.floatcc, "ne");
    let floatcc_uno = Literal::enumerator_for(&imm.floatcc, "uno");
    let floatcc_one = Literal::enumerator_for(&imm.floatcc, "one");

    // Equality needs an explicit `ord` test which checks the parity bit.
    group.legalize(
        def!(a = fcmp(floatcc_eq, x, y)),
        vec![
            def!(a1 = fcmp(floatcc_ord, x, y)),
            def!(a2 = fcmp(floatcc_ueq, x, y)),
            def!(a = band(a1, a2)),
        ],
    );
    group.legalize(
        def!(a = fcmp(floatcc_ne, x, y)),
        vec![
            def!(a1 = fcmp(floatcc_uno, x, y)),
            def!(a2 = fcmp(floatcc_one, x, y)),
            def!(a = bor(a1, a2)),
        ],
    );

    let floatcc_lt = &Literal::enumerator_for(&imm.floatcc, "lt");
    let floatcc_gt = &Literal::enumerator_for(&imm.floatcc, "gt");
    let floatcc_le = &Literal::enumerator_for(&imm.floatcc, "le");
    let floatcc_ge = &Literal::enumerator_for(&imm.floatcc, "ge");
    let floatcc_ugt = &Literal::enumerator_for(&imm.floatcc, "ugt");
    let floatcc_ult = &Literal::enumerator_for(&imm.floatcc, "ult");
    let floatcc_uge = &Literal::enumerator_for(&imm.floatcc, "uge");
    let floatcc_ule = &Literal::enumerator_for(&imm.floatcc, "ule");

    // Inequalities that need to be reversed.
    for &(cc, rev_cc) in &[
        (floatcc_lt, floatcc_gt),
        (floatcc_le, floatcc_ge),
        (floatcc_ugt, floatcc_ult),
        (floatcc_uge, floatcc_ule),
    ] {
        group.legalize(def!(a = fcmp(cc, x, y)), vec![def!(a = fcmp(rev_cc, y, x))]);
    }

    // We need to modify the CFG for min/max legalization.
    group.custom_legalize(fmin, "expand_minmax");
    group.custom_legalize(fmax, "expand_minmax");

    // Conversions from unsigned need special handling.
    group.custom_legalize(fcvt_from_uint, "expand_fcvt_from_uint");
    // Conversions from float to int can trap and modify the control flow graph.
    group.custom_legalize(fcvt_to_sint, "expand_fcvt_to_sint");
    group.custom_legalize(fcvt_to_uint, "expand_fcvt_to_uint");
    group.custom_legalize(fcvt_to_sint_sat, "expand_fcvt_to_sint_sat");
    group.custom_legalize(fcvt_to_uint_sat, "expand_fcvt_to_uint_sat");

    // Count leading and trailing zeroes, for baseline x86_64
    let c_minus_one = var("c_minus_one");
    let c_thirty_one = var("c_thirty_one");
    let c_thirty_two = var("c_thirty_two");
    let c_sixty_three = var("c_sixty_three");
    let c_sixty_four = var("c_sixty_four");
    let index1 = var("index1");
    let r2flags = var("r2flags");
    let index2 = var("index2");

    let intcc_eq = Literal::enumerator_for(&imm.intcc, "eq");
    let imm64_minus_one = Literal::constant(&imm.imm64, -1);
    let imm64_63 = Literal::constant(&imm.imm64, 63);
    group.legalize(
        def!(a = clz.I64(x)),
        vec![
            def!(c_minus_one = iconst(imm64_minus_one)),
            def!(c_sixty_three = iconst(imm64_63)),
            def!((index1, r2flags) = x86_bsr(x)),
            def!(index2 = selectif(intcc_eq, r2flags, c_minus_one, index1)),
            def!(a = isub(c_sixty_three, index2)),
        ],
    );

    let imm64_31 = Literal::constant(&imm.imm64, 31);
    group.legalize(
        def!(a = clz.I32(x)),
        vec![
            def!(c_minus_one = iconst(imm64_minus_one)),
            def!(c_thirty_one = iconst(imm64_31)),
            def!((index1, r2flags) = x86_bsr(x)),
            def!(index2 = selectif(intcc_eq, r2flags, c_minus_one, index1)),
            def!(a = isub(c_thirty_one, index2)),
        ],
    );

    let imm64_64 = Literal::constant(&imm.imm64, 64);
    group.legalize(
        def!(a = ctz.I64(x)),
        vec![
            def!(c_sixty_four = iconst(imm64_64)),
            def!((index1, r2flags) = x86_bsf(x)),
            def!(a = selectif(intcc_eq, r2flags, c_sixty_four, index1)),
        ],
    );

    let imm64_32 = Literal::constant(&imm.imm64, 32);
    group.legalize(
        def!(a = ctz.I32(x)),
        vec![
            def!(c_thirty_two = iconst(imm64_32)),
            def!((index1, r2flags) = x86_bsf(x)),
            def!(a = selectif(intcc_eq, r2flags, c_thirty_two, index1)),
        ],
    );

    // Population count for baseline x86_64
    let x = var("x");
    let r = var("r");

    let qv3 = var("qv3");
    let qv4 = var("qv4");
    let qv5 = var("qv5");
    let qv6 = var("qv6");
    let qv7 = var("qv7");
    let qv8 = var("qv8");
    let qv9 = var("qv9");
    let qv10 = var("qv10");
    let qv11 = var("qv11");
    let qv12 = var("qv12");
    let qv13 = var("qv13");
    let qv14 = var("qv14");
    let qv15 = var("qv15");
    let qc77 = var("qc77");
    #[allow(non_snake_case)]
    let qc0F = var("qc0F");
    let qc01 = var("qc01");

    let imm64_1 = Literal::constant(&imm.imm64, 1);
    let imm64_4 = Literal::constant(&imm.imm64, 4);
    group.legalize(
        def!(r = popcnt.I64(x)),
        vec![
            def!(qv3 = ushr_imm(x, imm64_1)),
            def!(qc77 = iconst(Literal::constant(&imm.imm64, 0x7777_7777_7777_7777))),
            def!(qv4 = band(qv3, qc77)),
            def!(qv5 = isub(x, qv4)),
            def!(qv6 = ushr_imm(qv4, imm64_1)),
            def!(qv7 = band(qv6, qc77)),
            def!(qv8 = isub(qv5, qv7)),
            def!(qv9 = ushr_imm(qv7, imm64_1)),
            def!(qv10 = band(qv9, qc77)),
            def!(qv11 = isub(qv8, qv10)),
            def!(qv12 = ushr_imm(qv11, imm64_4)),
            def!(qv13 = iadd(qv11, qv12)),
            def!(qc0F = iconst(Literal::constant(&imm.imm64, 0x0F0F_0F0F_0F0F_0F0F))),
            def!(qv14 = band(qv13, qc0F)),
            def!(qc01 = iconst(Literal::constant(&imm.imm64, 0x0101_0101_0101_0101))),
            def!(qv15 = imul(qv14, qc01)),
            def!(r = ushr_imm(qv15, Literal::constant(&imm.imm64, 56))),
        ],
    );

    let lv3 = var("lv3");
    let lv4 = var("lv4");
    let lv5 = var("lv5");
    let lv6 = var("lv6");
    let lv7 = var("lv7");
    let lv8 = var("lv8");
    let lv9 = var("lv9");
    let lv10 = var("lv10");
    let lv11 = var("lv11");
    let lv12 = var("lv12");
    let lv13 = var("lv13");
    let lv14 = var("lv14");
    let lv15 = var("lv15");
    let lc77 = var("lc77");
    #[allow(non_snake_case)]
    let lc0F = var("lc0F");
    let lc01 = var("lc01");

    group.legalize(
        def!(r = popcnt.I32(x)),
        vec![
            def!(lv3 = ushr_imm(x, imm64_1)),
            def!(lc77 = iconst(Literal::constant(&imm.imm64, 0x7777_7777))),
            def!(lv4 = band(lv3, lc77)),
            def!(lv5 = isub(x, lv4)),
            def!(lv6 = ushr_imm(lv4, imm64_1)),
            def!(lv7 = band(lv6, lc77)),
            def!(lv8 = isub(lv5, lv7)),
            def!(lv9 = ushr_imm(lv7, imm64_1)),
            def!(lv10 = band(lv9, lc77)),
            def!(lv11 = isub(lv8, lv10)),
            def!(lv12 = ushr_imm(lv11, imm64_4)),
            def!(lv13 = iadd(lv11, lv12)),
            def!(lc0F = iconst(Literal::constant(&imm.imm64, 0x0F0F_0F0F))),
            def!(lv14 = band(lv13, lc0F)),
            def!(lc01 = iconst(Literal::constant(&imm.imm64, 0x0101_0101))),
            def!(lv15 = imul(lv14, lc01)),
            def!(r = ushr_imm(lv15, Literal::constant(&imm.imm64, 24))),
        ],
    );

    group.build_and_add_to(&mut shared.transform_groups);

    let mut narrow = TransformGroupBuilder::new(
        "x86_narrow",
        r#"
    Legalize instructions by narrowing.

    Use x86-specific instructions if needed."#,
    )
    .isa("x86")
    .chain_with(shared.transform_groups.by_name("narrow_flags").id);

    // SIMD
    let uimm8_zero = Literal::constant(&imm.uimm8, 0x00);
    let uimm8_one = Literal::constant(&imm.uimm8, 0x01);
    let u128_zeroes = constant(vec![0x00; 16]);
    let b = var("b");
    let c = var("c");
    let d = var("d");
    let e = var("e");

    // SIMD vector size: eventually multiple vector sizes may be supported but for now only SSE-sized vectors are available
    let sse_vector_size: u64 = 128;
    let allowed_simd_type = |t: &LaneType| t.lane_bits() >= 8 && t.lane_bits() < 128;

    // SIMD splat: 8-bits
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 8) {
        let splat_any8x16 = splat.bind(vector(ty, sse_vector_size));
        narrow.legalize(
            def!(y = splat_any8x16(x)),
            vec![
                def!(a = scalar_to_vector(x)), // move into the lowest 8 bits of an XMM register
                def!(b = vconst(u128_zeroes)), // zero out a different XMM register; the shuffle mask
                // for moving the lowest byte to all other byte lanes is 0x0
                def!(y = x86_pshufb(a, b)), // PSHUFB takes two XMM operands, one of which is a
                                            // shuffle mask (i.e. b)
            ],
        );
    }

    // SIMD splat: 16-bits
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 16) {
        let splat_x16x8 = splat.bind(vector(ty, sse_vector_size));
        let raw_bitcast_any16x8_to_i32x4 = raw_bitcast
            .bind(vector(I32, sse_vector_size))
            .bind(vector(ty, sse_vector_size));
        let raw_bitcast_i32x4_to_any16x8 = raw_bitcast
            .bind(vector(ty, sse_vector_size))
            .bind(vector(I32, sse_vector_size));
        narrow.legalize(
            def!(y = splat_x16x8(x)),
            vec![
                def!(a = scalar_to_vector(x)), // move into the lowest 16 bits of an XMM register
                def!(b = insertlane(a, uimm8_one, x)), // insert the value again but in the next lowest 16 bits
                def!(c = raw_bitcast_any16x8_to_i32x4(b)), // no instruction emitted; pretend this is an I32x4 so we can use PSHUFD
                def!(d = x86_pshufd(c, uimm8_zero)), // broadcast the bytes in the XMM register with PSHUFD
                def!(y = raw_bitcast_i32x4_to_any16x8(d)), // no instruction emitted; pretend this is an X16x8 again
            ],
        );
    }

    // SIMD splat: 32-bits
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 32) {
        let splat_any32x4 = splat.bind(vector(ty, sse_vector_size));
        narrow.legalize(
            def!(y = splat_any32x4(x)),
            vec![
                def!(a = scalar_to_vector(x)), // translate to an x86 MOV to get the value in an XMM register
                def!(y = x86_pshufd(a, uimm8_zero)), // broadcast the bytes in the XMM register with PSHUF
            ],
        );
    }

    // SIMD splat: 64-bits
    for ty in ValueType::all_lane_types().filter(|t| t.lane_bits() == 64) {
        let splat_any64x2 = splat.bind(vector(ty, sse_vector_size));
        narrow.legalize(
            def!(y = splat_any64x2(x)),
            vec![
                def!(a = scalar_to_vector(x)), // move into the lowest 64 bits of an XMM register
                def!(y = insertlane(a, uimm8_one, x)), // move into the highest 64 bits of the same XMM register
            ],
        );
    }

    // SIMD bnot
    let ones = constant(vec![0xff; 16]);
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let bnot = bnot.bind(vector(ty, sse_vector_size));
        narrow.legalize(
            def!(y = bnot(x)),
            vec![def!(a = vconst(ones)), def!(y = bxor(a, x))],
        );
    }

    // SIMD shift left (logical)
    for ty in &[I16, I32, I64] {
        let ishl = ishl.bind(vector(*ty, sse_vector_size));
        let bitcast = bitcast.bind(vector(I64, sse_vector_size));
        narrow.legalize(
            def!(a = ishl(x, y)),
            vec![def!(b = bitcast(y)), def!(a = x86_psll(x, b))],
        );
    }

    // SIMD shift right (logical)
    for ty in &[I16, I32, I64] {
        let ushr = ushr.bind(vector(*ty, sse_vector_size));
        let bitcast = bitcast.bind(vector(I64, sse_vector_size));
        narrow.legalize(
            def!(a = ushr(x, y)),
            vec![def!(b = bitcast(y)), def!(a = x86_psrl(x, b))],
        );
    }

    // SIMD shift left (arithmetic)
    for ty in &[I16, I32, I64] {
        let sshr = sshr.bind(vector(*ty, sse_vector_size));
        let bitcast = bitcast.bind(vector(I64, sse_vector_size));
        narrow.legalize(
            def!(a = sshr(x, y)),
            vec![def!(b = bitcast(y)), def!(a = x86_psra(x, b))],
        );
    }

    // SIMD select
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let bitselect = bitselect.bind(vector(ty, sse_vector_size)); // must bind both x/y and c
        narrow.legalize(
            def!(d = bitselect(c, x, y)),
            vec![
                def!(a = band(x, c)),
                def!(b = band_not(y, c)),
                def!(d = bor(a, b)),
            ],
        );
    }

    // SIMD vany_true
    let ne = Literal::enumerator_for(&imm.intcc, "ne");
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let vany_true = vany_true.bind(vector(ty, sse_vector_size));
        narrow.legalize(
            def!(y = vany_true(x)),
            vec![def!(a = x86_ptest(x, x)), def!(y = trueif(ne, a))],
        );
    }

    // SIMD vall_true
    let eq = Literal::enumerator_for(&imm.intcc, "eq");
    for ty in ValueType::all_lane_types().filter(allowed_simd_type) {
        let vall_true = vall_true.bind(vector(ty, sse_vector_size));
        if ty.is_int() {
            // In the common case (Wasm's integer-only all_true), we do not require a bitcast.
            narrow.legalize(
                def!(y = vall_true(x)),
                vec![
                    def!(a = vconst(u128_zeroes)),
                    def!(c = icmp(eq, x, a)),
                    def!(d = x86_ptest(c, c)),
                    def!(y = trueif(eq, d)),
                ],
            );
        } else {
            // However, to support other types we must bitcast them to an integer vector to use
            // icmp.
            let lane_type_as_int = LaneType::int_from_bits(ty.lane_bits() as u16);
            let raw_bitcast_to_int = raw_bitcast.bind(vector(lane_type_as_int, sse_vector_size));
            narrow.legalize(
                def!(y = vall_true(x)),
                vec![
                    def!(a = vconst(u128_zeroes)),
                    def!(b = raw_bitcast_to_int(x)),
                    def!(c = icmp(eq, b, a)),
                    def!(d = x86_ptest(c, c)),
                    def!(y = trueif(eq, d)),
                ],
            );
        }
    }

    // SIMD icmp ne
    let ne = Literal::enumerator_for(&imm.intcc, "ne");
    for ty in ValueType::all_lane_types().filter(|ty| allowed_simd_type(ty) && ty.is_int()) {
        let icmp_ = icmp.bind(vector(ty, sse_vector_size));
        narrow.legalize(
            def!(c = icmp_(ne, a, b)),
            vec![def!(x = icmp(eq, a, b)), def!(c = bnot(x))],
        );
    }

    // SIMD icmp greater-/less-than
    let sgt = Literal::enumerator_for(&imm.intcc, "sgt");
    let ugt = Literal::enumerator_for(&imm.intcc, "ugt");
    let sge = Literal::enumerator_for(&imm.intcc, "sge");
    let uge = Literal::enumerator_for(&imm.intcc, "uge");
    let slt = Literal::enumerator_for(&imm.intcc, "slt");
    let ult = Literal::enumerator_for(&imm.intcc, "ult");
    let sle = Literal::enumerator_for(&imm.intcc, "sle");
    let ule = Literal::enumerator_for(&imm.intcc, "ule");
    for ty in &[I8, I16, I32] {
        // greater-than
        let icmp_ = icmp.bind(vector(*ty, sse_vector_size));
        narrow.legalize(
            def!(c = icmp_(ugt, a, b)),
            vec![def!(x = x86_pmaxu(a, b)), def!(c = icmp(eq, a, x))],
        );
        let icmp_ = icmp.bind(vector(*ty, sse_vector_size));
        narrow.legalize(
            def!(c = icmp_(sge, a, b)),
            vec![def!(x = x86_pmins(a, b)), def!(c = icmp(eq, x, b))],
        );
        let icmp_ = icmp.bind(vector(*ty, sse_vector_size));
        narrow.legalize(
            def!(c = icmp_(uge, a, b)),
            vec![def!(x = x86_pminu(a, b)), def!(c = icmp(eq, x, b))],
        );

        // less-than
        let icmp_ = icmp.bind(vector(*ty, sse_vector_size));
        narrow.legalize(def!(c = icmp_(slt, a, b)), vec![def!(c = icmp(sgt, b, a))]);
        let icmp_ = icmp.bind(vector(*ty, sse_vector_size));
        narrow.legalize(def!(c = icmp_(ult, a, b)), vec![def!(c = icmp(ugt, b, a))]);
        let icmp_ = icmp.bind(vector(*ty, sse_vector_size));
        narrow.legalize(def!(c = icmp_(sle, a, b)), vec![def!(c = icmp(sge, b, a))]);
        let icmp_ = icmp.bind(vector(*ty, sse_vector_size));
        narrow.legalize(def!(c = icmp_(ule, a, b)), vec![def!(c = icmp(uge, b, a))]);
    }

    for ty in &[F32, F64] {
        let fneg = fneg.bind(vector(*ty, sse_vector_size));
        let lane_type_as_int = LaneType::int_from_bits(LaneType::from(*ty).lane_bits() as u16);
        let uimm8_shift = Literal::constant(&imm.uimm8, lane_type_as_int.lane_bits() as i64 - 1);
        let vconst = vconst.bind(vector(lane_type_as_int, sse_vector_size));
        let bitcast_to_float = raw_bitcast.bind(vector(*ty, sse_vector_size));
        narrow.legalize(
            def!(b = fneg(a)),
            vec![
                def!(c = vconst(ones)),
                def!(d = ishl_imm(c, uimm8_shift)), // Create a mask of all 0s except the MSB.
                def!(e = bitcast_to_float(d)),      // Cast mask to the floating-point type.
                def!(b = bxor(a, e)),               // Flip the MSB.
            ],
        );
    }

    // SIMD fabs
    for ty in &[F32, F64] {
        let fabs = fabs.bind(vector(*ty, sse_vector_size));
        let lane_type_as_int = LaneType::int_from_bits(LaneType::from(*ty).lane_bits() as u16);
        let vconst = vconst.bind(vector(lane_type_as_int, sse_vector_size));
        let bitcast_to_float = raw_bitcast.bind(vector(*ty, sse_vector_size));
        narrow.legalize(
            def!(b = fabs(a)),
            vec![
                def!(c = vconst(ones)),
                def!(d = ushr_imm(c, uimm8_one)), // Create a mask of all 1s except the MSB.
                def!(e = bitcast_to_float(d)),    // Cast mask to the floating-point type.
                def!(b = band(a, e)),             // Unset the MSB.
            ],
        );
    }

    narrow.custom_legalize(shuffle, "convert_shuffle");
    narrow.custom_legalize(extractlane, "convert_extractlane");
    narrow.custom_legalize(insertlane, "convert_insertlane");
    narrow.custom_legalize(ineg, "convert_ineg");

    narrow.build_and_add_to(&mut shared.transform_groups);
}
