//! Encoding logic for EVEX instructions.

use crate::api::CodeSink;

/// EVEX prefix is always 4 bytes, byte 0 is 0x62
pub struct EvexPrefix {
    byte1: u8,
    byte2: u8,
    byte3: u8,
}

/// The EVEX prefix only ever uses the top bit (bit 3--the fourth bit) of any
/// HW-encoded register.
#[inline(always)]
fn invert_top_bit(enc: u8) -> u8 {
    (!(enc >> 3)) & 1
}

//         ┌───┬───┬───┬───┬───┬───┬───┬───┐
// Byte 1: │ R │ X │ B │ R'│ 0 │ 0 │ m │ m │
//         ├───┼───┼───┼───┼───┼───┼───┼───┤
// Byte 2: │ W │ v │ v │ v │ v │ 1 │ p │ p │
//         ├───┼───┼───┼───┼───┼───┼───┼───┤
// Byte 3: │ z │ L'│ L │ b │ V'│ a │ a │ a │
//         └───┴───┴───┴───┴───┴───┴───┴───┘

impl EvexPrefix {
    /// Construct the [`EvexPrefix`] for an instruction.
    pub fn new(
        reg: u8,
        vvvv: u8,
        (b, x): (Option<u8>, Option<u8>),
        ll: u8,
        pp: u8,
        mmm: u8,
        w: bool,
        broadcast: bool,
    ) -> Self {
        let r = invert_top_bit(reg);
        let r_prime = invert_top_bit(reg >> 1);
        let b = invert_top_bit(b.unwrap_or(0));
        let x = invert_top_bit(x.unwrap_or(0));
        let vvvv_value = !vvvv & 0b1111;
        let v_prime = !(vvvv >> 4) & 0b1;

        // byte1
        debug_assert!(mmm <= 0b111);
        let byte1 = r << 7 | x << 6 | b << 5 | r_prime << 4 | mmm;

        // byte2
        debug_assert!(vvvv <= 0b11111);
        debug_assert!(pp <= 0b11);
        let byte2 = (w as u8) << 7 | vvvv_value << 3 | 0b100 | (pp & 0b11);

        // byte3
        debug_assert!(ll < 0b11, "bits 11b are reserved (#UD); must fit in 2 bits");
        let aaa = 0b000; // Force k0 masking register for now; eventually this should be configurable (TODO).
        let z = 0; // Masking kind bit; not used yet (TODO) so we default to merge-masking.
        let byte3 = z | ll << 5 | (broadcast as u8) << 4 | v_prime << 3 | aaa;

        Self {
            byte1,
            byte2,
            byte3,
        }
    }

    /// Construct the [`EvexPrefix`] for an instruction.
    pub fn two_op(
        reg: u8,
        (b, x): (Option<u8>, Option<u8>),
        ll: u8,
        pp: u8,
        mmm: u8,
        w: bool,
        broadcast: bool,
    ) -> Self {
        EvexPrefix::new(reg, 0, (b, x), ll, pp, mmm, w, broadcast)
    }

    /// Construct the [`EvexPrefix`] for an instruction.
    pub fn three_op(
        reg: u8,
        vvvv: u8,
        (b, x): (Option<u8>, Option<u8>),
        ll: u8,
        pp: u8,
        mmm: u8,
        w: bool,
        broadcast: bool,
    ) -> Self {
        EvexPrefix::new(reg, vvvv, (b, x), ll, pp, mmm, w, broadcast)
    }

    // ---------------------------------------------------------------------
    // Intel APX "Extended EVEX" prefix for promoted *legacy* GPR instructions
    // (EVEX map 4). See the Intel APX Architecture Specification (rev 8),
    // section 3.1.2.3.1 "EVEX Extension of Legacy Instructions", Figure 3.3.
    //
    // The four bytes still begin with `0x62`, but several payload bits are
    // re-purposed relative to the AVX-512 layout above:
    //
    //         ┌────┬────┬────┬────┬────┬────┬────┬────┐
    // Byte 1: │ R3 │ X3 │ B3 │ R4 │ B4 │ 1  │ 0  │ 0  │  (map id = 4)
    //         ├────┼────┼────┼────┼────┼────┼────┼────┤
    // Byte 2: │ W  │ V3 │ V2 │ V1 │ V0 │ U  │ p  │ p  │  (U = ~X4)
    //         ├────┼────┼────┼────┼────┼────┼────┼────┤
    // Byte 3: │ 0  │ 0  │ 0  │ ND │ V4 │ NF │ 0  │ 0  │
    //         └────┴────┴────┴────┴────┴────┴────┴────┘
    //
    // The "underlined" fields (`R3`, `X3`, `B3`, `R4`, the `vvvv` bits and `V4`)
    // are stored inverted, exactly as in the AVX-512 layout. `B4` and `X4` are
    // newly repurposed reserved bits: `B4` uses *true* polarity (fixed value 0)
    // and `X4` is carried inverted in the `U` bit (`EVEX.X4 = ~EVEX.U`), so a
    // register-form instruction (ModRM.Mod = 3, no index) has `U = 1`.

    /// Construct the extended-EVEX (APX map 4) prefix for a legacy GPR
    /// instruction.
    ///
    /// `reg` is the ModRM.reg register, `vvvv` is the `V` register identifier
    /// (the NDD register when `nd` is set), and `(b, x)` are the ModRM.r/m base
    /// and (optional) SIB index registers. `nd`/`nf` select the New Data
    /// destination and No Flags bits respectively.
    pub fn legacy(
        reg: u8,
        vvvv: u8,
        (b, x): (Option<u8>, Option<u8>),
        pp: u8,
        mmm: u8,
        w: bool,
        nd: bool,
        nf: bool,
    ) -> Self {
        let base = b.unwrap_or(0);
        let index = x.unwrap_or(0);

        // byte1 (P0)
        let r3 = invert_top_bit(reg);
        let x3 = invert_top_bit(index);
        let b3 = invert_top_bit(base);
        let r4 = invert_top_bit(reg >> 1);
        let b4 = (base >> 4) & 1; // true polarity
        debug_assert!(mmm <= 0b111);
        let byte1 = r3 << 7 | x3 << 6 | b3 << 5 | r4 << 4 | b4 << 3 | mmm;

        // byte2 (P1)
        debug_assert!(vvvv <= 0b11111);
        debug_assert!(pp <= 0b11);
        let vvvv_value = !vvvv & 0b1111;
        // `EVEX.X4 = ~EVEX.U`; with no index register X4 = 0 so U = 1.
        let x4 = (index >> 4) & 1;
        let u = (!x4) & 1;
        let byte2 = (w as u8) << 7 | vvvv_value << 3 | u << 2 | (pp & 0b11);

        // byte3 (P2)
        let v_prime = invert_top_bit(vvvv >> 1); // V4, inverted
        let byte3 = (nd as u8) << 4 | v_prime << 3 | (nf as u8) << 2;

        Self {
            byte1,
            byte2,
            byte3,
        }
    }

    /// Construct the extended-EVEX (APX map 4) prefix for a two-operand legacy
    /// GPR instruction (no NDD); the `V` register identifier is unused.
    #[allow(dead_code, reason = "not all APX legacy forms are emitted yet")]
    pub fn legacy_two_op(
        reg: u8,
        (b, x): (Option<u8>, Option<u8>),
        pp: u8,
        mmm: u8,
        w: bool,
        nd: bool,
        nf: bool,
    ) -> Self {
        EvexPrefix::legacy(reg, 0, (b, x), pp, mmm, w, nd, nf)
    }

    /// Construct the extended-EVEX (APX map 4) prefix for a three-operand
    /// legacy GPR instruction; `vvvv` carries the NDD register.
    pub fn legacy_three_op(
        reg: u8,
        vvvv: u8,
        (b, x): (Option<u8>, Option<u8>),
        pp: u8,
        mmm: u8,
        w: bool,
        nd: bool,
        nf: bool,
    ) -> Self {
        EvexPrefix::legacy(reg, vvvv, (b, x), pp, mmm, w, nd, nf)
    }

    pub(crate) fn encode(&self, sink: &mut impl CodeSink) {
        sink.put1(0x62);
        sink.put1(self.byte1);
        sink.put1(self.byte2);
        sink.put1(self.byte3);
    }
}
