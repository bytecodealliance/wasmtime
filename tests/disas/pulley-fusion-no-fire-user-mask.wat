;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--funcs all"

;; Phase 1 / phase 2 fusion gating against user wasm: the recogniser
;; gates on `imm.bits() == -2`, which would naively match the wat
;; `(i32.const -2) (i32.and) (br_if)` user pattern and risk a soundness
;; mismatch (the fused op tests UNMASKED src for non-zero, whereas the
;; original brif tests `(v & -2) != 0` — they differ at v == 1).
;;
;; The bug is unreachable from wasm because:
;;   * `br_if` cond is always i32 (wasm validation), AND
;;   * the wat parser stores `(i32.const -2)` as `Imm64(0xFFFFFFFE)`
;;     (= 4294967294), NOT `Imm64(-2)`.
;; So `imm.bits() == -2` doesn't match the wat-emitted i32 form. The
;; only producer of `Imm64(-2)` reaching the recogniser is
;; `func_environ::get_or_init_func_ref_table_elem`'s call to
;; `Imm64::from(-2_i64)`.
;;
;; This test pins the surface behaviour. If the gate ever changes to
;; accept i32 -2 encodings too, the disas would suddenly start
;; containing `xband32_s8_br_if_*` or `xfuncref_dispatch_*` here, and
;; this test fails — that's the signal to re-audit soundness.

(module
  (func (export "test") (param $v i32) (result i32) (local $tmp i32)
    local.get $v
    i32.const -2
    i32.and
    local.tee $tmp
    local.get $tmp
    br_if 0
    drop
    i32.const 999))
;; wasm[0]::function[0]:
;;       push_frame
;;       xband32_s8 x0, x2, -2
;;       br_if32 x0, 0xa    // target = 0xf
;;    b: xconst16 x0, 999
;;       pop_frame
;;       ret
;;
;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xload32le_o32 x14, x2, 0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x15, x0, 8
;;       xmov_fp x2
;;       xstore64le_o32 x15, 72, x2
;;       xmov x2, sp
;;       xstore64le_o32 x15, 64, x2
;;       xpcadd x2, 0x2d    // target = 0x6d
;;       xstore64le_o32 x15, 80, x2
;;       call3 x0, x1, x14, -0x4f    // target = 0x0
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x6d
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;   6d: xzero x0
;;   6f: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;   74: ret
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       push_frame_save 32, x16, x17
;;       xmov x3, x0
;;       xmov x17, x1
;;       xload64le_o32 x14, x1, 8
;;       xmov_fp x15
;;       xstore64le_o32 x14, 48, x15
;;       xmov_lr x15
;;       xstore64le_o32 x14, 56, x15
;;       xmov x16, sp
;;       xstore32le_o32 x16, 0, x2
;;       xload64le_o32 x0, x0, 8
;;       xone x4
;;       xmov x1, x3
;;       xmov x2, x17
;;       xmov x3, x16
;;       call_indirect_host 0
;;       zext8 x0, x0
;;       br_if_not32 x0, 0x13    // target = 0xd3
;;   c6: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;   d3: xmov x1, x17
;;   d6: xload64le_o32 x0, x1, 16
;;   dd: xload64le_o32 x0, x0, 408
;;   e4: call_indirect_host 52
;;   e8: trap
