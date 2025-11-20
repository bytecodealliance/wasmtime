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
;;       b.lo    #0x15c
;;   44: stur    x2, [sp]
;;       stur    x2, [sp, #0x10]
;;       mov     w22, #0x2a
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 61, slot at FP-0xc0, locals , stack 
;;       stur    w22, [sp, #8]
;;       stur    w22, [sp, #8]
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 64, slot at FP-0xc0, locals , stack 
;;       ldur    x2, [sp, #0x10]
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 66, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       bl      #0x3f0
;;   60: mov     x24, x2
;;       mov     w3, #0x4000000
;;       mov     w4, #2
;;       mov     w5, #0x28
;;       mov     w6, #8
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x310
;;   7c: ldur    x13, [sp, #0x10]
;;       ldr     x5, [x13, #8]
;;       ldr     x6, [x5, #0x18]
;;       stur    x5, [sp, #0x20]
;;       add     x15, x6, #0x20
;;       str     w22, [x15, w2, uxtw]
;;       add     x3, x6, #0x18
;;       mov     x7, x24
;;       str     w7, [x3, w2, uxtw]
;;       mov     w3, #0
;;       add     x4, x6, #0x1c
;;       stur    x6, [sp, #0x18]
;;       str     w3, [x4, w2, uxtw]
;;       mov     x3, x2
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x428
;;       ├─╼ exception frame offset: SP = FP - 0xc0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0xd4
;;   bc: mov     w3, #9
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x384
;;   c8: ldur    x2, [sp, #0x10]
;;       bl      #0x3bc
;;   d0: .byte   0x1f, 0xc1, 0x00, 0x00
;;       mov     x15, x0
;;       mov     w6, w15
;;       mov     x7, #0x28
;;       adds    x5, x6, x7
;;       cset    x7, hs
;;       uxtb    w6, w7
;;       cbnz    x6, #0x174
;;   f0: ldur    x12, [sp, #0x20]
;;       ldr     x8, [x12, #0x20]
;;       cmp     x5, x8
;;       cset    x9, hi
;;       uxtb    w9, w9
;;       cbnz    x9, #0x178
;;  108: ldur    x6, [sp, #0x18]
;;       add     x10, x6, #0x20
;;       ldr     w12, [x10, w15, uxtw]
;;       stur    w12, [sp, #8]
;;       ldur    x2, [sp, #0x10]
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 72, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ldr     x14, [x2, #0x30]
;;       ldr     x2, [x2, #0x40]
;;       ldur    x3, [sp, #0x10]
;;       blr     x14
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 72, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;  12c: add     sp, sp, #0x30
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 75, slot at FP-0xc0, locals , stack 
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
;;  15c: stur    x2, [sp, #0x10]
;;  160: mov     w3, #0
;;  164: bl      #0x384
;;  168: ldur    x2, [sp, #0x10]
;;  16c: bl      #0x3bc
;;  170: .byte   0x1f, 0xc1, 0x00, 0x00
;;  174: .byte   0x1f, 0xc1, 0x00, 0x00
;;  178: .byte   0x1f, 0xc1, 0x00, 0x00
