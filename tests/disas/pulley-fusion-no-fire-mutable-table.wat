;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--funcs all"

;; Phase 1 / phase 2 fusion gating: a single `table.set` anywhere in
;; the module sets `tables_mutated[idx] = true` for that table, which
;; disables the `is_eagerly_initialized_funcref_table` predicate.
;; func_environ's IR rewrite then emits the ORIGINAL brif on `value`
;; (unmasked) instead of the rewritten brif on `value_masked`. With no
;; `brif(band(v, -2))` pattern reaching the lowering, neither phase 1
;; (BandBrIf) nor phase 2 (FuncrefDispatch) fires. The dispatch tail
;; keeps its separate band + brif + xload + xload + call_indirect ops.
;;
;; Reference precedents in upstream interpreters where similar
;; mutation-invariant edges caused real bugs:
;;   - V8 issue 5913 (call_indirect signature mismatch under table
;;     sharing) — the sig-elide invariant must not survive a foreign
;;     mutation.
;;   - GHSA-q49f-xg75-m9xw (wasmtime Winch table.fill host panic) —
;;     bulk table ops must invalidate fusion-eligibility.
;;   - Hermes 24a8fe64 (HiddenClass GC'd mid-IC), Luau release/717
;;     (userdata write didn't invalidate store cache) — the general
;;     shape "fused-op cached state survives invalidation source".
;;
;; This test pins the gating. Adding a `table.set` anywhere should
;; produce the unfused dispatch sequence below.

(module
  (table 3 3 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  ;; Mutator: clears the immutability proof for table 0.
  (func (export "mutate") (param i32)
    local.get 0
    ref.func $f1
    table.set 0)

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
;;       push_frame_save 16, x16, x17
;;       xmov x12, x0
;;       xmov x17, x2
;;       xzero x9
;;       xmov x16, x12
;;       call2 x16, x9, 0x3da    // target = 0x3fb
;;       xmov x2, x17
;;       xmov x12, x16
;;       br_if_xugteq32_u8 x2, 3, 0x2b    // target = 0x59
;;   35: xbor64_s8 x10, x0, 1
;;       xmov x0, x12
;;       xload64le_o32 x11, x0, 48
;;       zext32 x12, x2
;;       xshl64_u6 x12, x12, 3
;;       xadd64 x11, x11, x12
;;       xstore64le_o32 x11, 0, x10
;;       pop_frame_restore 16, x16, x17
;;       ret
;;   59: trap
;;       ╰─╼ trap: Normal(TableOutOfBounds)
;;
;; wasm[0]::function[4]:
;;       push_frame_save 16, x28
;;       xmov x3, x0
;;       br_if_xugteq32_u8 x2, 3, 0x7c    // target = 0xe0
;;   6b: xmov x1, x3
;;       xload64le_o32 x0, x1, 48
;;       zext32 x1, x2
;;       xshl64_u6 x2, x1, 3
;;       xadd64 x0, x0, x2
;;       xload64le_o32 x2, x0, 0
;;       xband64_s8 x0, x2, -2
;;       br_if_xeq64_i8 x2, 0, 0x45    // target = 0xce
;;   90: xmov x28, x3
;;       br_if_xeq64_i8 x0, 0, 0x50    // target = 0xe3
;;   9a: xload32le_o32 x1, x0, 16
;;       xload64le_o32 x2, x28, 40
;;       xload32le_o32 x2, x2, 0
;;       br_if_xneq32 x1, x2, 0x37    // target = 0xe6
;;   b6: xload64le_o32 x1, x0, 8
;;       xload64le_o32 x0, x0, 24
;;       call_indirect2 x1, x0, x28
;;       pop_frame_restore 16, x28
;;       ret
;;   ce: xzero x0
;;   d0: xmov x28, x3
;;   d3: call3 x28, x0, x1, 0x363    // target = 0x436
;;   db: jump -0x48    // target = 0x93
;;   e0: trap
;;       ╰─╼ trap: Normal(TableOutOfBounds)
;;   e3: trap
;;       ╰─╼ trap: Normal(IndirectCallToNull)
;;   e6: trap
;;       ╰─╼ trap: Normal(BadSignature)
;;
;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x13, x0, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 72, x14
;;       xmov x14, sp
;;       xstore64le_o32 x13, 64, x14
;;       xpcadd x15, 0x2a    // target = 0x13b
;;       xstore64le_o32 x13, 80, x15
;;       call -0x120    // target = 0x0
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x13b
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  13b: xzero x0
;;  13d: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  142: ret
;;
;; wasm[0]::array_to_wasm_trampoline[1]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x13, x0, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 72, x14
;;       xmov x14, sp
;;       xstore64le_o32 x13, 64, x14
;;       xpcadd x15, 0x2a    // target = 0x195
;;       xstore64le_o32 x13, 80, x15
;;       call -0x175    // target = 0x5
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x195
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  195: xzero x0
;;  197: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  19c: ret
;;
;; wasm[0]::array_to_wasm_trampoline[2]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x13, x0, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 72, x14
;;       xmov x14, sp
;;       xstore64le_o32 x13, 64, x14
;;       xpcadd x15, 0x2a    // target = 0x1ef
;;       xstore64le_o32 x13, 80, x15
;;       call -0x1c9    // target = 0xb
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x1ef
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  1ef: xzero x0
;;  1f1: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  1f6: ret
;;
;; wasm[0]::array_to_wasm_trampoline[3]:
;;       push_frame_save 128, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xload32le_o32 x13, x2, 0
;;       xload64le_o32 x14, x0, 8
;;       xmov_fp x15
;;       xstore64le_o32 x14, 72, x15
;;       xmov x15, sp
;;       xstore64le_o32 x14, 64, x15
;;       xpcadd x15, 0x1f    // target = 0x23e
;;       xstore64le_o32 x14, 80, x15
;;       call3 x0, x1, x13, -0x21d    // target = 0x11
;;       ├─╼ exception frame offset: SP = FP - 0x80
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x23e
;;       xone x0
;;       pop_frame_restore 128, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  23e: xzero x0
;;  240: pop_frame_restore 128, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  245: ret
;;
;; wasm[0]::array_to_wasm_trampoline[4]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xload32le_o32 x14, x2, 0
;;       xstore64le_o32 sp, 0, x2
;;       xload64le_o32 x15, x0, 8
;;       xmov_fp x2
;;       xstore64le_o32 x15, 72, x2
;;       xmov x2, sp
;;       xstore64le_o32 x15, 64, x2
;;       xpcadd x2, 0x2d    // target = 0x2a2
;;       xstore64le_o32 x15, 80, x2
;;       call3 x0, x1, x14, -0x228    // target = 0x5c
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x2a2
;;       xload64le_o32 x2, sp, 0
;;       xstore32le_o32 x2, 0, x0
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;  2a2: xzero x0
;;  2a4: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;  2a9: ret
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
;;       br_if_not32 x15, 0x13    // target = 0x301
;;  2f4: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;  301: xmov x1, x17
;;  304: xload64le_o32 x0, x1, 16
;;  30b: xload64le_o32 x0, x0, 328
;;  312: call_indirect_host 42
;;  316: trap
;;
;; signatures[1]::wasm_to_array_trampoline:
;;       push_frame_save 32, x16
;;       xmov x5, x0
;;       xmov x16, x1
;;       xload64le_o32 x13, x1, 8
;;       xmov_fp x14
;;       xstore64le_o32 x13, 48, x14
;;       xmov_lr x14
;;       xstore64le_o32 x13, 56, x14
;;       xmov x3, sp
;;       xstore32le_o32 x3, 0, x2
;;       xload64le_o32 x0, x0, 8
;;       xone x4
;;       xmov x1, x5
;;       xmov x2, x16
;;       call_indirect_host 0
;;       zext8 x0, x0
;;       br_if_not32 x0, 0xc    // target = 0x36d
;;  367: pop_frame_restore 32, x16
;;       ret
;;  36d: xmov x1, x16
;;  370: xload64le_o32 x0, x1, 16
;;  377: xload64le_o32 x0, x0, 328
;;  37e: call_indirect_host 42
;;  382: trap
;;
;; signatures[2]::wasm_to_array_trampoline:
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
;;       br_if_not32 x0, 0x13    // target = 0x3e3
;;  3d6: xload32le_o32 x0, x16, 0
;;       pop_frame_restore 32, x16, x17
;;       ret
;;  3e3: xmov x1, x17
;;  3e6: xload64le_o32 x0, x1, 16
;;  3ed: xload64le_o32 x0, x0, 328
;;  3f4: call_indirect_host 42
;;  3f8: trap
;;
;; wasmtime_builtin_ref_func:
;;       push_frame
;;       xload64le_o32 x8, x0, 8
;;       xmov_fp x9
;;       xstore64le_o32 x8, 48, x9
;;       xmov_lr x9
;;       xstore64le_o32 x8, 56, x9
;;       xload64le_o32 x10, x0, 16
;;       xmov x11, x0
;;       xload64le_o32 x0, x10, 48
;;       xmov x2, x1
;;       xmov x1, x11
;;       call_indirect_host 7
;;       pop_frame
;;       ret
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
;;       xload64le_o32 x0, x11, 56
;;       xmov x3, x2
;;       xmov x2, x1
;;       xmov x1, x13
;;       call_indirect_host 8
;;       pop_frame
;;       ret
