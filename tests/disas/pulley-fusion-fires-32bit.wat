;;! target = "pulley32"
;;! test = "compile"
;;! objdump = "--funcs all"

;; Phase 2 fusion on 32-bit Pulley (used by arm64_32-apple-watchos
;; via cross-language LTO + linker-plugin-lto). The fused op is
;; `xfuncref_dispatch_x32` with i8 offsets 4 (wasm_call) and 12
;; (vmctx) — half of the pulley64 offsets (8 and 24).
;;
;; This test pins the 32-bit dispatch tail shape AND verifies that
;; the `imm.bits() == -2` gate fires here (the band's Imm64 from
;; func_environ's `Imm64::from(-2_i64)` still bits-equals -2 even
;; though Cranelift truncates the imm to i32 for an i32 band).
;;
;; Known-follow-up from `docs/opcode-fusion-funcref-dispatch.md` →
;; "Known follow-ups" — arm64_32 / Apple Watch confirmation. This
;; test is the static side of that confirmation; the dynamic side
;; (a Pulley-on-Apple-Watch run) is gated by Apple Watch SE2
;; hardware access.

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
;;       br_if_xugteq32_u8 x2, 3, 0x58    // target = 0x6e
;;   1d: xload32le_o32 x15, x0, 24
;;       xmov x3, x0
;;       xshl32_u6 x0, x2, 2
;;       xadd32 x15, x15, x0
;;       xload32le_o32 x15, x15, 0
;;       xband32_s8 x0, x15, -2
;;       br_if_not32 x15, 0x21    // target = 0x59
;;   3e: xmov x25, x3
;;       xload32le_o32 x1, x0, 4
;;       xload32le_o32 x0, x0, 12
;;       call_indirect2 x1, x0, x25
;;       pop_frame_restore 16, x25
;;       ret
;;   59: xzero x0
;;   5b: zext32 x1, x2
;;   5e: xmov x25, x3
;;   61: call3 x25, x0, x1, 0x267    // target = 0x2c8
;;   69: jump -0x28    // target = 0x41
;;   6e: trap
;;       ╰─╼ trap: Normal(TableOutOfBounds)
;;
;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload32le_o32 x13, x0, 4
;;       xmov_fp x14
;;       xstore32le_o32 x13, 48, x14
;;       xmov x14, sp
;;       xstore32le_o32 x13, 44, x14
;;       xpcadd x15, 0x2a    // target = 0xc3
;;       xstore32le_o32 x13, 52, x15
;;       call -0xa8    // target = 0x0
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xc3
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;   c3: xzero x0
;;   c5: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;   ca: ret
;;
;; wasm[0]::array_to_wasm_trampoline[1]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload32le_o32 x13, x0, 4
;;       xmov_fp x14
;;       xstore32le_o32 x13, 48, x14
;;       xmov x14, sp
;;       xstore32le_o32 x13, 44, x14
;;       xpcadd x15, 0x2a    // target = 0x11d
;;       xstore32le_o32 x13, 52, x15
;;       call -0xfd    // target = 0x5
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x11d
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  11d: xzero x0
;;  11f: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  124: ret
;;
;; wasm[0]::array_to_wasm_trampoline[2]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload32le_o32 x13, x0, 4
;;       xmov_fp x14
;;       xstore32le_o32 x13, 48, x14
;;       xmov x14, sp
;;       xstore32le_o32 x13, 44, x14
;;       xpcadd x15, 0x2a    // target = 0x177
;;       xstore32le_o32 x13, 52, x15
;;       call -0x151    // target = 0xb
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x177
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  177: xzero x0
;;  179: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  17e: ret
;;
;; wasm[0]::array_to_wasm_trampoline[3]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xload32le_o32 x14, x2, 0
;;       xstore64le_o32 sp, 0, x2
;;       xload32le_o32 x15, x0, 4
;;       xmov_fp x2
;;       xstore32le_o32 x15, 48, x2
;;       xmov x2, sp
;;       xstore32le_o32 x15, 44, x2
;;       xpcadd x2, 0x2d    // target = 0x1db
;;       xstore32le_o32 x15, 52, x2
;;       call3 x0, x1, x14, -0x1ac    // target = 0x11
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x1db
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  1db: xzero x0
;;  1dd: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  1e2: ret
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       push_frame_save 32, x16, x17
;;       xmov x2, x0
;;       xmov x17, x1
;;       xload32le_o32 x13, x1, 4
;;       xmov_fp x14
;;       xstore32le_o32 x13, 36, x14
;;       xmov_lr x14
;;       xstore32le_o32 x13, 40, x14
;;       xload32le_o32 x0, x0, 4
;;       xmov x16, sp
;;       xone x4
;;       xmov x1, x2
;;       xmov x2, x17
;;       xmov x3, x16
;;       call_indirect_host 0
;;       zext8 x15, x0
;;       br_if_not32 x15, 0x13    // target = 0x23a
;;  22d: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;  23a: xmov x1, x17
;;  23d: xload32le_o32 x0, x1, 8
;;  244: xload32le_o32 x0, x0, 164
;;  24b: call_indirect_host 42
;;  24f: trap
;;
;; signatures[1]::wasm_to_array_trampoline:
;;       push_frame_save 32, x16, x17
;;       xmov x3, x0
;;       xmov x17, x1
;;       xload32le_o32 x14, x1, 4
;;       xmov_fp x15
;;       xstore32le_o32 x14, 36, x15
;;       xmov_lr x15
;;       xstore32le_o32 x14, 40, x15
;;       xmov x16, sp
;;       xstore32le_o32 x16, 0, x2
;;       xload32le_o32 x0, x0, 4
;;       xone x4
;;       xmov x1, x3
;;       xmov x2, x17
;;       xmov x3, x16
;;       call_indirect_host 0
;;       zext8 x0, x0
;;       br_if_not32 x0, 0x13    // target = 0x2b0
;;  2a3: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;  2b0: xmov x1, x17
;;  2b3: xload32le_o32 x0, x1, 8
;;  2ba: xload32le_o32 x0, x0, 164
;;  2c1: call_indirect_host 42
;;  2c5: trap
;;
;; wasmtime_builtin_table_get_lazy_init_func_ref:
;;       push_frame
;;       xload32le_o32 x9, x0, 4
;;       xmov_fp x10
;;       xstore32le_o32 x9, 36, x10
;;       xmov_lr x10
;;       xstore32le_o32 x9, 40, x10
;;       xload32le_o32 x11, x0, 8
;;       xmov x13, x0
;;       xload32le_o32 x0, x11, 28
;;       xmov x3, x2
;;       xmov x2, x1
;;       xmov x1, x13
;;       call_indirect_host 8
;;       pop_frame
;;       ret
