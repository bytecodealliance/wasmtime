;;! target = "aarch64"
;;! test = "compile"
;;! flags = ["-Wexceptions=yes", "-Wgc=yes", "-Dguest-debug=yes"]

(module
  (tag $t (param i32))
  (import "" "host" (func))
  (func (export "main")
    (block $b (result i32)
      (try_table (catch $t $b)
        (drop (i32.const 42))
        (throw $t (i32.const 42)))
      i32.const 0)
    (call 0)
    (drop)))
;; wasm[0]::function[1]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       stp     x27, x28, [sp, #-0x10]!
;;       stp     x25, x26, [sp, #-0x10]!
;;       stp     x23, x24, [sp, #-0x10]!
;;       stp     x21, x22, [sp, #-0x10]!
;;       stp     x19, x20, [sp, #-0x10]!
;;       stp     d14, d15, [sp, #-0x10]!
;;       stp     d12, d13, [sp, #-0x10]!
;;       stp     d10, d11, [sp, #-0x10]!
;;       stp     d8, d9, [sp, #-0x10]!
;;       sub     sp, sp, #0x30
;;       ldr     x0, [x2, #8]
;;       ldr     x0, [x0, #0x18]
;;       mov     x1, sp
;;       cmp     x1, x0
;;       b.lo    #0x194
;;   44: stur    x2, [sp]
;;       mov     x0, x2
;;       stur    x2, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 53, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 53, patch bytes [38, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 55, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 55, patch bytes [36, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 61, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 61, patch bytes [34, 1, 0, 148]
;;       mov     w19, #0x2a
;;       stur    w19, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 63, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 63, patch bytes [31, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 64, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 64, patch bytes [30, 1, 0, 148]
;;       stur    w19, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 66, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 66, patch bytes [28, 1, 0, 148]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x448
;;   84: mov     x20, x2
;;       mov     w3, #0x4000000
;;       mov     w4, #2
;;       mov     w5, #0x28
;;       mov     w6, #8
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x368
;;   a0: ldur    x0, [sp, #0x10]
;;       ldr     x5, [x0, #8]
;;       ldr     x6, [x5, #0x20]
;;       stur    x5, [sp, #0x20]
;;       add     x3, x6, #0x20
;;       str     w19, [x3, w2, uxtw]
;;       add     x3, x6, #0x18
;;       mov     x0, x20
;;       str     w0, [x3, w2, uxtw]
;;       mov     w3, #0
;;       add     x4, x6, #0x1c
;;       stur    x6, [sp, #0x18]
;;       str     w3, [x4, w2, uxtw]
;;       mov     x3, x2
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x480
;;       ├─╼ exception frame offset: SP = FP - 0xc0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0xf8
;;   e0: mov     w3, #9
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x3dc
;;   ec: ldur    x2, [sp, #0x10]
;;       bl      #0x414
;;   f4: .byte   0x1f, 0xc1, 0x00, 0x00
;;       mov     x2, x0
;;       mov     w3, w2
;;       mov     x4, #0x28
;;       adds    x3, x3, x4
;;       cset    x4, hs
;;       uxtb    w4, w4
;;       cbnz    x4, #0x1ac
;;  114: ldur    x5, [sp, #0x20]
;;       ldr     x1, [x5, #0x28]
;;       cmp     x3, x1
;;       cset    x1, hi
;;       uxtb    w1, w1
;;       cbnz    x1, #0x1b0
;;  12c: ldur    x6, [sp, #0x18]
;;       add     x0, x6, #0x20
;;       ldr     w0, [x0, w2, uxtw]
;;       stur    w0, [sp, #8]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 72, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 72, patch bytes [234, 0, 0, 148]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x0, [x1, #0x30]
;;       ldr     x2, [x1, #0x40]
;;       ldur    x3, [sp, #0x10]
;;       blr     x0
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 72, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;  158: ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 74, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 74, patch bytes [227, 0, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 75, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 75, patch bytes [226, 0, 0, 148]
;;       add     sp, sp, #0x30
;;       ldp     d8, d9, [sp], #0x10
;;       ldp     d10, d11, [sp], #0x10
;;       ldp     d12, d13, [sp], #0x10
;;       ldp     d14, d15, [sp], #0x10
;;       ldp     x19, x20, [sp], #0x10
;;       ldp     x21, x22, [sp], #0x10
;;       ldp     x23, x24, [sp], #0x10
;;       ldp     x25, x26, [sp], #0x10
;;       ldp     x27, x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  194: stur    x2, [sp, #0x10]
;;  198: mov     w3, #0
;;  19c: bl      #0x3dc
;;  1a0: ldur    x2, [sp, #0x10]
;;  1a4: bl      #0x414
;;  1a8: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1ac: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1b0: .byte   0x1f, 0xc1, 0x00, 0x00
