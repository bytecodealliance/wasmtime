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
;;       ldr     x3, [x2, #8]
;;       ldr     x3, [x3, #0x10]
;;       mov     x4, sp
;;       cmp     x4, x3
;;       b.lo    #0x190
;;   44: stur    x2, [sp]
;;       mov     x0, x2
;;       stur    x2, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 53, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 53, patch bytes [29, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 55, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 55, patch bytes [27, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 61, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 61, patch bytes [25, 1, 0, 148]
;;       mov     w22, #0x2a
;;       stur    w22, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 63, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 63, patch bytes [22, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 64, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 64, patch bytes [21, 1, 0, 148]
;;       stur    w22, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 66, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 66, patch bytes [19, 1, 0, 148]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x424
;;   84: mov     x24, x2
;;       mov     w3, #0x4000000
;;       mov     w4, #2
;;       mov     w5, #0x28
;;       mov     w6, #8
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x344
;;   a0: ldur    x0, [sp, #0x10]
;;       ldr     x5, [x0, #8]
;;       ldr     x6, [x5, #0x18]
;;       stur    x5, [sp, #0x20]
;;       add     x15, x6, #0x20
;;       str     w22, [x15, w2, uxtw]
;;       add     x3, x6, #0x18
;;       mov     x1, x24
;;       str     w1, [x3, w2, uxtw]
;;       mov     w3, #0
;;       add     x4, x6, #0x1c
;;       stur    x6, [sp, #0x18]
;;       str     w3, [x4, w2, uxtw]
;;       mov     x3, x2
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x45c
;;       ├─╼ exception frame offset: SP = FP - 0xc0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0xf8
;;   e0: mov     w3, #9
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x3b8
;;   ec: ldur    x2, [sp, #0x10]
;;       bl      #0x3f0
;;   f4: .byte   0x1f, 0xc1, 0x00, 0x00
;;       mov     x9, x0
;;       mov     w6, w9
;;       mov     x7, #0x28
;;       adds    x5, x6, x7
;;       cset    x7, hs
;;       uxtb    w6, w7
;;       cbnz    x6, #0x1a8
;;  114: ldur    x4, [sp, #0x20]
;;       ldr     x8, [x4, #0x20]
;;       cmp     x5, x8
;;       cset    x10, hi
;;       uxtb    w10, w10
;;       cbnz    x10, #0x1ac
;;  12c: ldur    x6, [sp, #0x18]
;;       add     x10, x6, #0x20
;;       ldr     w12, [x10, w9, uxtw]
;;       stur    w12, [sp, #8]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 72, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 72, patch bytes [225, 0, 0, 148]
;;       ldr     x14, [x0, #0x30]
;;       ldr     x2, [x0, #0x40]
;;       ldur    x3, [sp, #0x10]
;;       blr     x14
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 72, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;  154: ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 74, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 74, patch bytes [219, 0, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 75, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 75, patch bytes [218, 0, 0, 148]
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
;;  190: stur    x2, [sp, #0x10]
;;  194: mov     w3, #0
;;  198: bl      #0x3b8
;;  19c: ldur    x2, [sp, #0x10]
;;  1a0: bl      #0x3f0
;;  1a4: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1a8: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1ac: .byte   0x1f, 0xc1, 0x00, 0x00
