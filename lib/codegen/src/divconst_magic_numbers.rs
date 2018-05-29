//! Compute "magic numbers" for division-by-constants transformations.
//!
//! Math helpers for division by (non-power-of-2) constants. This is based
//! on the presentation in "Hacker's Delight" by Henry Warren, 2003. There
//! are four cases: {unsigned, signed} x {32 bit, 64 bit}. The word size
//! makes little difference, but the signed-vs-unsigned aspect has a large
//! effect. Therefore everything is presented in the order U32 U64 S32 S64
//! so as to emphasise the similarity of the U32 and U64 cases and the S32
//! and S64 cases.

#![allow(non_snake_case)]

// Structures to hold the "magic numbers" computed.

#[derive(PartialEq, Debug)]
pub struct MU32 {
    pub mulBy: u32,
    pub doAdd: bool,
    pub shiftBy: i32,
}

#[derive(PartialEq, Debug)]
pub struct MU64 {
    pub mulBy: u64,
    pub doAdd: bool,
    pub shiftBy: i32,
}

#[derive(PartialEq, Debug)]
pub struct MS32 {
    pub mulBy: i32,
    pub shiftBy: i32,
}

#[derive(PartialEq, Debug)]
pub struct MS64 {
    pub mulBy: i64,
    pub shiftBy: i32,
}

// The actual "magic number" generators follow.

pub fn magicU32(d: u32) -> MU32 {
    debug_assert_ne!(d, 0);
    debug_assert_ne!(d, 1); // d==1 generates out of range shifts.

    let mut do_add: bool = false;
    let mut p: i32 = 31;
    let nc: u32 = 0xFFFFFFFFu32 - u32::wrapping_neg(d) % d;
    let mut q1: u32 = 0x80000000u32 / nc;
    let mut r1: u32 = 0x80000000u32 - q1 * nc;
    let mut q2: u32 = 0x7FFFFFFFu32 / d;
    let mut r2: u32 = 0x7FFFFFFFu32 - q2 * d;
    loop {
        p = p + 1;
        if r1 >= nc - r1 {
            q1 = u32::wrapping_add(u32::wrapping_mul(2, q1), 1);
            r1 = u32::wrapping_sub(u32::wrapping_mul(2, r1), nc);
        } else {
            q1 = 2 * q1;
            r1 = 2 * r1;
        }
        if r2 + 1 >= d - r2 {
            if q2 >= 0x7FFFFFFFu32 {
                do_add = true;
            }
            q2 = 2 * q2 + 1;
            r2 = u32::wrapping_sub(u32::wrapping_add(u32::wrapping_mul(2, r2), 1), d);
        } else {
            if q2 >= 0x80000000u32 {
                do_add = true;
            }
            q2 = u32::wrapping_mul(2, q2);
            r2 = 2 * r2 + 1;
        }
        let delta: u32 = d - 1 - r2;
        if !(p < 64 && (q1 < delta || (q1 == delta && r1 == 0))) {
            break;
        }
    }

    MU32 {
        mulBy: q2 + 1,
        doAdd: do_add,
        shiftBy: p - 32,
    }
}

pub fn magicU64(d: u64) -> MU64 {
    debug_assert_ne!(d, 0);
    debug_assert_ne!(d, 1); // d==1 generates out of range shifts.

    let mut do_add: bool = false;
    let mut p: i32 = 63;
    let nc: u64 = 0xFFFFFFFFFFFFFFFFu64 - u64::wrapping_neg(d) % d;
    let mut q1: u64 = 0x8000000000000000u64 / nc;
    let mut r1: u64 = 0x8000000000000000u64 - q1 * nc;
    let mut q2: u64 = 0x7FFFFFFFFFFFFFFFu64 / d;
    let mut r2: u64 = 0x7FFFFFFFFFFFFFFFu64 - q2 * d;
    loop {
        p = p + 1;
        if r1 >= nc - r1 {
            q1 = u64::wrapping_add(u64::wrapping_mul(2, q1), 1);
            r1 = u64::wrapping_sub(u64::wrapping_mul(2, r1), nc);
        } else {
            q1 = 2 * q1;
            r1 = 2 * r1;
        }
        if r2 + 1 >= d - r2 {
            if q2 >= 0x7FFFFFFFFFFFFFFFu64 {
                do_add = true;
            }
            q2 = 2 * q2 + 1;
            r2 = u64::wrapping_sub(u64::wrapping_add(u64::wrapping_mul(2, r2), 1), d);
        } else {
            if q2 >= 0x8000000000000000u64 {
                do_add = true;
            }
            q2 = u64::wrapping_mul(2, q2);
            r2 = 2 * r2 + 1;
        }
        let delta: u64 = d - 1 - r2;
        if !(p < 128 && (q1 < delta || (q1 == delta && r1 == 0))) {
            break;
        }
    }

    MU64 {
        mulBy: q2 + 1,
        doAdd: do_add,
        shiftBy: p - 64,
    }
}

pub fn magicS32(d: i32) -> MS32 {
    debug_assert_ne!(d, -1);
    debug_assert_ne!(d, 0);
    debug_assert_ne!(d, 1);
    let two31: u32 = 0x80000000u32;
    let mut p: i32 = 31;
    let ad: u32 = i32::wrapping_abs(d) as u32;
    let t: u32 = two31 + ((d as u32) >> 31);
    let anc: u32 = u32::wrapping_sub(t - 1, t % ad);
    let mut q1: u32 = two31 / anc;
    let mut r1: u32 = two31 - q1 * anc;
    let mut q2: u32 = two31 / ad;
    let mut r2: u32 = two31 - q2 * ad;
    loop {
        p = p + 1;
        q1 = 2 * q1;
        r1 = 2 * r1;
        if r1 >= anc {
            q1 = q1 + 1;
            r1 = r1 - anc;
        }
        q2 = 2 * q2;
        r2 = 2 * r2;
        if r2 >= ad {
            q2 = q2 + 1;
            r2 = r2 - ad;
        }
        let delta: u32 = ad - r2;
        if !(q1 < delta || (q1 == delta && r1 == 0)) {
            break;
        }
    }

    MS32 {
        mulBy: (if d < 0 {
            u32::wrapping_neg(q2 + 1)
        } else {
            q2 + 1
        }) as i32,
        shiftBy: p - 32,
    }
}

pub fn magicS64(d: i64) -> MS64 {
    debug_assert_ne!(d, -1);
    debug_assert_ne!(d, 0);
    debug_assert_ne!(d, 1);
    let two63: u64 = 0x8000000000000000u64;
    let mut p: i32 = 63;
    let ad: u64 = i64::wrapping_abs(d) as u64;
    let t: u64 = two63 + ((d as u64) >> 63);
    let anc: u64 = u64::wrapping_sub(t - 1, t % ad);
    let mut q1: u64 = two63 / anc;
    let mut r1: u64 = two63 - q1 * anc;
    let mut q2: u64 = two63 / ad;
    let mut r2: u64 = two63 - q2 * ad;
    loop {
        p = p + 1;
        q1 = 2 * q1;
        r1 = 2 * r1;
        if r1 >= anc {
            q1 = q1 + 1;
            r1 = r1 - anc;
        }
        q2 = 2 * q2;
        r2 = 2 * r2;
        if r2 >= ad {
            q2 = q2 + 1;
            r2 = r2 - ad;
        }
        let delta: u64 = ad - r2;
        if !(q1 < delta || (q1 == delta && r1 == 0)) {
            break;
        }
    }

    MS64 {
        mulBy: (if d < 0 {
            u64::wrapping_neg(q2 + 1)
        } else {
            q2 + 1
        }) as i64,
        shiftBy: p - 64,
    }
}

#[cfg(test)]
mod tests {
    use super::{magicS32, magicS64, magicU32, magicU64};
    use super::{MS32, MS64, MU32, MU64};

    fn mkMU32(mulBy: u32, doAdd: bool, shiftBy: i32) -> MU32 {
        MU32 {
            mulBy,
            doAdd,
            shiftBy,
        }
    }

    fn mkMU64(mulBy: u64, doAdd: bool, shiftBy: i32) -> MU64 {
        MU64 {
            mulBy,
            doAdd,
            shiftBy,
        }
    }

    fn mkMS32(mulBy: i32, shiftBy: i32) -> MS32 {
        MS32 { mulBy, shiftBy }
    }

    fn mkMS64(mulBy: i64, shiftBy: i32) -> MS64 {
        MS64 { mulBy, shiftBy }
    }

    #[test]
    fn test_magicU32() {
        assert_eq!(magicU32(2u32), mkMU32(0x80000000u32, false, 0));
        assert_eq!(magicU32(3u32), mkMU32(0xaaaaaaabu32, false, 1));
        assert_eq!(magicU32(4u32), mkMU32(0x40000000u32, false, 0));
        assert_eq!(magicU32(5u32), mkMU32(0xcccccccdu32, false, 2));
        assert_eq!(magicU32(6u32), mkMU32(0xaaaaaaabu32, false, 2));
        assert_eq!(magicU32(7u32), mkMU32(0x24924925u32, true, 3));
        assert_eq!(magicU32(9u32), mkMU32(0x38e38e39u32, false, 1));
        assert_eq!(magicU32(10u32), mkMU32(0xcccccccdu32, false, 3));
        assert_eq!(magicU32(11u32), mkMU32(0xba2e8ba3u32, false, 3));
        assert_eq!(magicU32(12u32), mkMU32(0xaaaaaaabu32, false, 3));
        assert_eq!(magicU32(25u32), mkMU32(0x51eb851fu32, false, 3));
        assert_eq!(magicU32(125u32), mkMU32(0x10624dd3u32, false, 3));
        assert_eq!(magicU32(625u32), mkMU32(0xd1b71759u32, false, 9));
        assert_eq!(magicU32(1337u32), mkMU32(0x88233b2bu32, true, 11));
        assert_eq!(magicU32(65535u32), mkMU32(0x80008001u32, false, 15));
        assert_eq!(magicU32(65536u32), mkMU32(0x00010000u32, false, 0));
        assert_eq!(magicU32(65537u32), mkMU32(0xffff0001u32, false, 16));
        assert_eq!(magicU32(31415927u32), mkMU32(0x445b4553u32, false, 23));
        assert_eq!(magicU32(0xdeadbeefu32), mkMU32(0x93275ab3u32, false, 31));
        assert_eq!(magicU32(0xfffffffdu32), mkMU32(0x40000001u32, false, 30));
        assert_eq!(magicU32(0xfffffffeu32), mkMU32(0x00000003u32, true, 32));
        assert_eq!(magicU32(0xffffffffu32), mkMU32(0x80000001u32, false, 31));
    }
    #[test]
    fn test_magicU64() {
        assert_eq!(magicU64(2u64), mkMU64(0x8000000000000000u64, false, 0));
        assert_eq!(magicU64(3u64), mkMU64(0xaaaaaaaaaaaaaaabu64, false, 1));
        assert_eq!(magicU64(4u64), mkMU64(0x4000000000000000u64, false, 0));
        assert_eq!(magicU64(5u64), mkMU64(0xcccccccccccccccdu64, false, 2));
        assert_eq!(magicU64(6u64), mkMU64(0xaaaaaaaaaaaaaaabu64, false, 2));
        assert_eq!(magicU64(7u64), mkMU64(0x2492492492492493u64, true, 3));
        assert_eq!(magicU64(9u64), mkMU64(0xe38e38e38e38e38fu64, false, 3));
        assert_eq!(magicU64(10u64), mkMU64(0xcccccccccccccccdu64, false, 3));
        assert_eq!(magicU64(11u64), mkMU64(0x2e8ba2e8ba2e8ba3u64, false, 1));
        assert_eq!(magicU64(12u64), mkMU64(0xaaaaaaaaaaaaaaabu64, false, 3));
        assert_eq!(magicU64(25u64), mkMU64(0x47ae147ae147ae15u64, true, 5));
        assert_eq!(magicU64(125u64), mkMU64(0x0624dd2f1a9fbe77u64, true, 7));
        assert_eq!(magicU64(625u64), mkMU64(0x346dc5d63886594bu64, false, 7));
        assert_eq!(magicU64(1337u64), mkMU64(0xc4119d952866a139u64, false, 10));
        assert_eq!(
            magicU64(31415927u64),
            mkMU64(0x116d154b9c3d2f85u64, true, 25)
        );
        assert_eq!(
            magicU64(0x00000000deadbeefu64),
            mkMU64(0x93275ab2dfc9094bu64, false, 31)
        );
        assert_eq!(
            magicU64(0x00000000fffffffdu64),
            mkMU64(0x8000000180000005u64, false, 31)
        );
        assert_eq!(
            magicU64(0x00000000fffffffeu64),
            mkMU64(0x0000000200000005u64, true, 32)
        );
        assert_eq!(
            magicU64(0x00000000ffffffffu64),
            mkMU64(0x8000000080000001u64, false, 31)
        );
        assert_eq!(
            magicU64(0x0000000100000000u64),
            mkMU64(0x0000000100000000u64, false, 0)
        );
        assert_eq!(
            magicU64(0x0000000100000001u64),
            mkMU64(0xffffffff00000001u64, false, 32)
        );
        assert_eq!(
            magicU64(0x0ddc0ffeebadf00du64),
            mkMU64(0x2788e9d394b77da1u64, true, 60)
        );
        assert_eq!(
            magicU64(0xfffffffffffffffdu64),
            mkMU64(0x4000000000000001u64, false, 62)
        );
        assert_eq!(
            magicU64(0xfffffffffffffffeu64),
            mkMU64(0x0000000000000003u64, true, 64)
        );
        assert_eq!(
            magicU64(0xffffffffffffffffu64),
            mkMU64(0x8000000000000001u64, false, 63)
        );
    }
    #[test]
    fn test_magicS32() {
        assert_eq!(magicS32(-0x80000000i32), mkMS32(0x7fffffffu32 as i32, 30));
        assert_eq!(magicS32(-0x7FFFFFFFi32), mkMS32(0xbfffffffu32 as i32, 29));
        assert_eq!(magicS32(-0x7FFFFFFEi32), mkMS32(0x7ffffffdu32 as i32, 30));
        assert_eq!(magicS32(-31415927i32), mkMS32(0xbba4baadu32 as i32, 23));
        assert_eq!(magicS32(-1337i32), mkMS32(0x9df73135u32 as i32, 9));
        assert_eq!(magicS32(-256i32), mkMS32(0x7fffffffu32 as i32, 7));
        assert_eq!(magicS32(-5i32), mkMS32(0x99999999u32 as i32, 1));
        assert_eq!(magicS32(-3i32), mkMS32(0x55555555u32 as i32, 1));
        assert_eq!(magicS32(-2i32), mkMS32(0x7fffffffu32 as i32, 0));
        assert_eq!(magicS32(2i32), mkMS32(0x80000001u32 as i32, 0));
        assert_eq!(magicS32(3i32), mkMS32(0x55555556u32 as i32, 0));
        assert_eq!(magicS32(4i32), mkMS32(0x80000001u32 as i32, 1));
        assert_eq!(magicS32(5i32), mkMS32(0x66666667u32 as i32, 1));
        assert_eq!(magicS32(6i32), mkMS32(0x2aaaaaabu32 as i32, 0));
        assert_eq!(magicS32(7i32), mkMS32(0x92492493u32 as i32, 2));
        assert_eq!(magicS32(9i32), mkMS32(0x38e38e39u32 as i32, 1));
        assert_eq!(magicS32(10i32), mkMS32(0x66666667u32 as i32, 2));
        assert_eq!(magicS32(11i32), mkMS32(0x2e8ba2e9u32 as i32, 1));
        assert_eq!(magicS32(12i32), mkMS32(0x2aaaaaabu32 as i32, 1));
        assert_eq!(magicS32(25i32), mkMS32(0x51eb851fu32 as i32, 3));
        assert_eq!(magicS32(125i32), mkMS32(0x10624dd3u32 as i32, 3));
        assert_eq!(magicS32(625i32), mkMS32(0x68db8badu32 as i32, 8));
        assert_eq!(magicS32(1337i32), mkMS32(0x6208cecbu32 as i32, 9));
        assert_eq!(magicS32(31415927i32), mkMS32(0x445b4553u32 as i32, 23));
        assert_eq!(magicS32(0x7ffffffei32), mkMS32(0x80000003u32 as i32, 30));
        assert_eq!(magicS32(0x7fffffffi32), mkMS32(0x40000001u32 as i32, 29));
    }
    #[test]
    fn test_magicS64() {
        assert_eq!(
            magicS64(-0x8000000000000000i64),
            mkMS64(0x7fffffffffffffffu64 as i64, 62)
        );
        assert_eq!(
            magicS64(-0x7FFFFFFFFFFFFFFFi64),
            mkMS64(0xbfffffffffffffffu64 as i64, 61)
        );
        assert_eq!(
            magicS64(-0x7FFFFFFFFFFFFFFEi64),
            mkMS64(0x7ffffffffffffffdu64 as i64, 62)
        );
        assert_eq!(
            magicS64(-0x0ddC0ffeeBadF00di64),
            mkMS64(0x6c3b8b1635a4412fu64 as i64, 59)
        );
        assert_eq!(
            magicS64(-0x100000001i64),
            mkMS64(0x800000007fffffffu64 as i64, 31)
        );
        assert_eq!(
            magicS64(-0x100000000i64),
            mkMS64(0x7fffffffffffffffu64 as i64, 31)
        );
        assert_eq!(
            magicS64(-0xFFFFFFFFi64),
            mkMS64(0x7fffffff7fffffffu64 as i64, 31)
        );
        assert_eq!(
            magicS64(-0xFFFFFFFEi64),
            mkMS64(0x7ffffffefffffffdu64 as i64, 31)
        );
        assert_eq!(
            magicS64(-0xFFFFFFFDi64),
            mkMS64(0x7ffffffe7ffffffbu64 as i64, 31)
        );
        assert_eq!(
            magicS64(-0xDeadBeefi64),
            mkMS64(0x6cd8a54d2036f6b5u64 as i64, 31)
        );
        assert_eq!(
            magicS64(-31415927i64),
            mkMS64(0x7749755a31e1683du64 as i64, 24)
        );
        assert_eq!(magicS64(-1337i64), mkMS64(0x9df731356bccaf63u64 as i64, 9));
        assert_eq!(magicS64(-256i64), mkMS64(0x7fffffffffffffffu64 as i64, 7));
        assert_eq!(magicS64(-5i64), mkMS64(0x9999999999999999u64 as i64, 1));
        assert_eq!(magicS64(-3i64), mkMS64(0x5555555555555555u64 as i64, 1));
        assert_eq!(magicS64(-2i64), mkMS64(0x7fffffffffffffffu64 as i64, 0));
        assert_eq!(magicS64(2i64), mkMS64(0x8000000000000001u64 as i64, 0));
        assert_eq!(magicS64(3i64), mkMS64(0x5555555555555556u64 as i64, 0));
        assert_eq!(magicS64(4i64), mkMS64(0x8000000000000001u64 as i64, 1));
        assert_eq!(magicS64(5i64), mkMS64(0x6666666666666667u64 as i64, 1));
        assert_eq!(magicS64(6i64), mkMS64(0x2aaaaaaaaaaaaaabu64 as i64, 0));
        assert_eq!(magicS64(7i64), mkMS64(0x4924924924924925u64 as i64, 1));
        assert_eq!(magicS64(9i64), mkMS64(0x1c71c71c71c71c72u64 as i64, 0));
        assert_eq!(magicS64(10i64), mkMS64(0x6666666666666667u64 as i64, 2));
        assert_eq!(magicS64(11i64), mkMS64(0x2e8ba2e8ba2e8ba3u64 as i64, 1));
        assert_eq!(magicS64(12i64), mkMS64(0x2aaaaaaaaaaaaaabu64 as i64, 1));
        assert_eq!(magicS64(25i64), mkMS64(0xa3d70a3d70a3d70bu64 as i64, 4));
        assert_eq!(magicS64(125i64), mkMS64(0x20c49ba5e353f7cfu64 as i64, 4));
        assert_eq!(magicS64(625i64), mkMS64(0x346dc5d63886594bu64 as i64, 7));
        assert_eq!(magicS64(1337i64), mkMS64(0x6208ceca9433509du64 as i64, 9));
        assert_eq!(
            magicS64(31415927i64),
            mkMS64(0x88b68aa5ce1e97c3u64 as i64, 24)
        );
        assert_eq!(
            magicS64(0x00000000deadbeefi64),
            mkMS64(0x93275ab2dfc9094bu64 as i64, 31)
        );
        assert_eq!(
            magicS64(0x00000000fffffffdi64),
            mkMS64(0x8000000180000005u64 as i64, 31)
        );
        assert_eq!(
            magicS64(0x00000000fffffffei64),
            mkMS64(0x8000000100000003u64 as i64, 31)
        );
        assert_eq!(
            magicS64(0x00000000ffffffffi64),
            mkMS64(0x8000000080000001u64 as i64, 31)
        );
        assert_eq!(
            magicS64(0x0000000100000000i64),
            mkMS64(0x8000000000000001u64 as i64, 31)
        );
        assert_eq!(
            magicS64(0x0000000100000001i64),
            mkMS64(0x7fffffff80000001u64 as i64, 31)
        );
        assert_eq!(
            magicS64(0x0ddc0ffeebadf00di64),
            mkMS64(0x93c474e9ca5bbed1u64 as i64, 59)
        );
        assert_eq!(
            magicS64(0x7ffffffffffffffdi64),
            mkMS64(0x2000000000000001u64 as i64, 60)
        );
        assert_eq!(
            magicS64(0x7ffffffffffffffei64),
            mkMS64(0x8000000000000003u64 as i64, 62)
        );
        assert_eq!(
            magicS64(0x7fffffffffffffffi64),
            mkMS64(0x4000000000000001u64 as i64, 61)
        );
    }
    #[test]
    fn test_magic_generators_dont_panic() {
        // The point of this is to check that the magic number generators
        // don't panic with integer wraparounds, especially at boundary
        // cases for their arguments. The actual results are thrown away.
        let mut total: u64 = 0;
        // Testing UP magicU32
        for x in 2..(200 * 1000u32) {
            let m = magicU32(x);
            total = total ^ (m.mulBy as u64);
            total = total + (m.shiftBy as u64);
            total = total - (if m.doAdd { 123 } else { 456 });
        }
        assert_eq!(total, 1747815691);
        // Testing DOWN magicU32
        for x in 0..(200 * 1000u32) {
            let m = magicU32(0xFFFF_FFFFu32 - x);
            total = total ^ (m.mulBy as u64);
            total = total + (m.shiftBy as u64);
            total = total - (if m.doAdd { 123 } else { 456 });
        }
        assert_eq!(total, 2210292772);

        // Testing UP magicU64
        for x in 2..(200 * 1000u64) {
            let m = magicU64(x);
            total = total ^ m.mulBy;
            total = total + (m.shiftBy as u64);
            total = total - (if m.doAdd { 123 } else { 456 });
        }
        assert_eq!(total, 7430004084791260605);
        // Testing DOWN magicU64
        for x in 0..(200 * 1000u64) {
            let m = magicU64(0xFFFF_FFFF_FFFF_FFFFu64 - x);
            total = total ^ m.mulBy;
            total = total + (m.shiftBy as u64);
            total = total - (if m.doAdd { 123 } else { 456 });
        }
        assert_eq!(total, 7547519887519825919);

        // Testing UP magicS32
        for x in 0..(200 * 1000i32) {
            let m = magicS32(-0x8000_0000i32 + x);
            total = total ^ (m.mulBy as u64);
            total = total + (m.shiftBy as u64);
        }
        assert_eq!(total, 10899224186731671235);
        // Testing DOWN magicS32
        for x in 0..(200 * 1000i32) {
            let m = magicS32(0x7FFF_FFFFi32 - x);
            total = total ^ (m.mulBy as u64);
            total = total + (m.shiftBy as u64);
        }
        assert_eq!(total, 7547519887517897369);

        // Testing UP magicS64
        for x in 0..(200 * 1000i64) {
            let m = magicS64(-0x8000_0000_0000_0000i64 + x);
            total = total ^ (m.mulBy as u64);
            total = total + (m.shiftBy as u64);
        }
        assert_eq!(total, 8029756891368555163);
        // Testing DOWN magicS64
        for x in 0..(200 * 1000i64) {
            let m = magicS64(0x7FFF_FFFF_FFFF_FFFFi64 - x);
            total = total ^ (m.mulBy as u64);
            total = total + (m.shiftBy as u64);
        }
        // Force `total` -- and hence, the entire computation -- to
        // be used, so that rustc can't optimise it out.
        assert_eq!(total, 7547519887532559585u64);
    }
}
