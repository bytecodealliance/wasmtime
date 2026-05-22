;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--funcs all"

;; Immutable funcref table fully populated by a static elem segment — the
;; `is_eagerly_initialized_funcref_table` predicate holds. The Cranelift
;; Pulley backend rewrites the call_indirect lazy-init brif to test the
;; masked funcref value, then `pulley_shared::lower::try_fuse_band_brif`
;; folds the band-imm + brif into one `xband64_s8_br_if_x64` Pulley op.
;;
;; What we pin here: the fused op appears in the disassembly of the
;; call_indirect dispatch tail, and the standalone `xband64_s8` that
;; would otherwise produce the masked value is gone (absorbed by the
;; fused op via `Lower::sink_pure_inst`).

(module
  (table 3 3 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

  (elem (i32.const 0) func $f1 $f2 $f3))
;; wasm[0]::function[0]::f1:
;;       push_frame
;;       xone x0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]::f2:
;;       push_frame
;;       xconst8 x0, 2
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[2]::f3:
;;       push_frame
;;       xconst8 x0, 3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[3]:
;;       push_frame_save 16, x25
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 3, 0x54    // target = 0x6d
;;   20: xload64le_o32 x0, x0, 48
;;       zext32 x15, x2
;;       xshl64_u6 x1, x15, 3
;;       xadd64 x0, x0, x1
;;       xload64le_o32 x0, x0, 0
;;       xband64_s8_br_if_not_x64 x0, x0, -2, 0x24    // target = 0x5b
;;   3f: xmov x25, x3
;;       xload64le_o32 x2, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       xmov x1, x25
;;       call_indirect x2
;;       pop_frame_restore 16, x25
;;       ret
;;   5b: xzero x0
;;   5d: xmov x25, x3
;;   60: call3 x25, x0, x15, 0x267    // target = 0x2c7
;;   68: jump -0x26    // target = 0x42
;;   6d: trap
;;       ╰─╼ trap: TableOutOfBounds
;;
;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x13, x0, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 72, x14
;;       xmov x14, sp
;;       xstore64le_o32 x13, 64, x14
;;       xpcadd x15, 0x2a    // target = 0xc2
;;       xstore64le_o32 x13, 80, x15
;;       call -0xa7    // target = 0x0
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xc2
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;   c2: xzero x0
;;   c4: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;   c9: ret
;;
;; wasm[0]::array_to_wasm_trampoline[1]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x13, x0, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 72, x14
;;       xmov x14, sp
;;       xstore64le_o32 x13, 64, x14
;;       xpcadd x15, 0x2a    // target = 0x11c
;;       xstore64le_o32 x13, 80, x15
;;       call -0xfc    // target = 0x5
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x11c
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  11c: xzero x0
;;  11e: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  123: ret
;;
;; wasm[0]::array_to_wasm_trampoline[2]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x13, x0, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 72, x14
;;       xmov x14, sp
;;       xstore64le_o32 x13, 64, x14
;;       xpcadd x15, 0x2a    // target = 0x176
;;       xstore64le_o32 x13, 80, x15
;;       call -0x150    // target = 0xb
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x176
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  176: xzero x0
;;  178: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  17d: ret
;;
;; wasm[0]::array_to_wasm_trampoline[3]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xload32le_o32 x14, x2, 0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x15, x0, 8
;;       xmov_fp x2
;;       xstore64le_o32 x15, 72, x2
;;       xmov x2, sp
;;       xstore64le_o32 x15, 64, x2
;;       xpcadd x2, 0x2d    // target = 0x1da
;;       xstore64le_o32 x15, 80, x2
;;       call3 x0, x1, x14, -0x1ab    // target = 0x11
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x1da
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  1da: xzero x0
;;  1dc: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  1e1: ret
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       push_frame_save 32, x16, x17
;;       xmov x2, x0
;;       xmov x17, x1
;;       xload64le_o32 x13, x1, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 48, x14
;;       xmov_lr x14
;;       xstore64le_o32 x13, 56, x14
;;       xload64le_o32 x0, x0, 8
;;       xmov x16, sp
;;       xone x4
;;       xmov x1, x2
;;       xmov x2, x17
;;       xmov x3, x16
;;       call_indirect_host 0
;;       zext8 x15, x0
;;       br_if_not32 x15, 0x13    // target = 0x239
;;  22c: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;  239: xmov x1, x17
;;  23c: xload64le_o32 x0, x1, 16
;;  243: xload64le_o32 x0, x0, 408
;;  24a: call_indirect_host 52
;;  24e: trap
;;
;; signatures[1]::wasm_to_array_trampoline:
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
;;       br_if_not32 x0, 0x13    // target = 0x2af
;;  2a2: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;  2af: xmov x1, x17
;;  2b2: xload64le_o32 x0, x1, 16
;;  2b9: xload64le_o32 x0, x0, 408
;;  2c0: call_indirect_host 52
;;  2c4: trap
;;
;; wasmtime_builtin_table_get_lazy_init_func_ref:
;;       push_frame
;;       xload64le_o32 x9, x0, 8
;;       xmov_fp x10
;;       xstore64le_o32 x9, 48, x10
;;       xmov_lr x10
;;       xstore64le_o32 x9, 56, x10
;;       xload64le_o32 x11, x0, 16
;;       xmov x13, x0
;;       xload64le_o32 x0, x11, 72
;;       xmov x3, x2
;;       xmov x2, x1
;;       xmov x1, x13
;;       call_indirect_host 10
;;       pop_frame
;;       ret
