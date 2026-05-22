;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--funcs all"

;; Immutable funcref table fully populated by a static elem segment — the
;; `is_eagerly_initialized_funcref_table` predicate holds AND sig check
;; is statically elided. Two-layer fusion fires at the call_indirect
;; dispatch tail:
;;
;;   1. `try_fuse_funcref_dispatch` (phase 2) absorbs the brif + the two
;;      VMFuncRef field loads (`wasm_call` + `vmctx`) emitted by
;;      `load_code_and_vmctx`, and emits one `xfuncref_dispatch_not_x64`
;;      Pulley op. The continuation block's standalone loads are skipped
;;      via the cross-block sink performed by Pulley's `pre_lower` hook.
;;
;;   2. The preceding `xband64_s8 v, -2` stays as a separate op (its
;;      result is `src` to the fused dispatch). Phase-1's `BandBrIf`
;;      fusion does NOT fire here because phase 2 absorbs the brif
;;      first (the recogniser tries phase 2 before phase 1).
;;
;; What we pin here: the dispatch tail is exactly
;; `xband64_s8 ; xfuncref_dispatch_not_x64 ; call_indirect` — three
;; Pulley dispatches instead of the unfused five.

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
;;       push_frame_save 32, x16, x17, x26
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 3, 0x58    // target = 0x71
;;   20: xload64le_o32 x0, x0, 48
;;       zext32 x15, x2
;;       xshl64_u6 x1, x15, 3
;;       xadd64 x0, x0, x1
;;       xload64le_o32 x0, x0, 0
;;       xband64_s8 x0, x0, -2
;;       xfuncref_dispatch_not_x64 x16, x17, x0, 8, 24, 0x1b    // target = 0x56
;;       xmov x2, x0
;;       xmov x1, x3
;;       xmov x0, x17
;;       call_indirect x16
;;       pop_frame_restore 32, x16, x17, x26
;;       ret
;;   56: xzero x0
;;   58: xmov x26, x3
;;   5b: call3 x26, x0, x15, 0x270    // target = 0x2cb
;;   63: xmov x2, x0
;;   66: xmov x0, x17
;;   69: xmov x1, x26
;;   6c: jump -0x1e    // target = 0x4e
;;   71: trap
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
;;       xpcadd x15, 0x2a    // target = 0xc6
;;       xstore64le_o32 x13, 80, x15
;;       call -0xab    // target = 0x0
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xc6
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;   c6: xzero x0
;;   c8: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;   cd: ret
;;
;; wasm[0]::array_to_wasm_trampoline[1]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x13, x0, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 72, x14
;;       xmov x14, sp
;;       xstore64le_o32 x13, 64, x14
;;       xpcadd x15, 0x2a    // target = 0x120
;;       xstore64le_o32 x13, 80, x15
;;       call -0x100    // target = 0x5
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x120
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  120: xzero x0
;;  122: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  127: ret
;;
;; wasm[0]::array_to_wasm_trampoline[2]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x13, x0, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 72, x14
;;       xmov x14, sp
;;       xstore64le_o32 x13, 64, x14
;;       xpcadd x15, 0x2a    // target = 0x17a
;;       xstore64le_o32 x13, 80, x15
;;       call -0x154    // target = 0xb
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x17a
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  17a: xzero x0
;;  17c: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  181: ret
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
;;       xpcadd x2, 0x2d    // target = 0x1de
;;       xstore64le_o32 x15, 80, x2
;;       call3 x0, x1, x14, -0x1af    // target = 0x11
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x1de
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  1de: xzero x0
;;  1e0: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  1e5: ret
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
;;       br_if_not32 x15, 0x13    // target = 0x23d
;;  230: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;  23d: xmov x1, x17
;;  240: xload64le_o32 x0, x1, 16
;;  247: xload64le_o32 x0, x0, 408
;;  24e: call_indirect_host 52
;;  252: trap
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
;;       br_if_not32 x0, 0x13    // target = 0x2b3
;;  2a6: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;  2b3: xmov x1, x17
;;  2b6: xload64le_o32 x0, x1, 16
;;  2bd: xload64le_o32 x0, x0, 408
;;  2c4: call_indirect_host 52
;;  2c8: trap
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
