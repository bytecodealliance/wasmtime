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
;;       sub     sp, sp, #0x20
;;       ldr     x0, [x2, #8]
;;       ldr     x0, [x0, #0x18]
;;       mov     x1, sp
;;       cmp     x1, x0
;;       b.lo    #0x21c
;;   44: stur    x2, [sp]
;;       mov     x0, x2
;;       stur    x2, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x35, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x35, patch bytes [69, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x37, patch bytes [67, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3d, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x3d, patch bytes [65, 1, 0, 148]
;;       mov     w1, #0x2a
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3f, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x3f, patch bytes [62, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x40, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x40, patch bytes [61, 1, 0, 148]
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x42, patch bytes [59, 1, 0, 148]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x4c4
;;   84: ldur    x0, [sp, #0x10]
;;       mov     x19, x2
;;       ldr     x2, [x0, #0x20]
;;       ldr     w3, [x2]
;;       mov     w4, w3
;;       add     x4, x4, #0x20
;;       ldr     w5, [x2, #4]
;;       cmp     x4, x5
;;       b.hi    #0x1b4
;;   a8: add     w5, w3, #0x20
;;       str     w5, [x2]
;;       mov     w7, #0x4000000
;;       ldur    x0, [sp, #0x10]
;;       ldr     x6, [x0, #8]
;;       ldr     x6, [x6, #0x20]
;;       add     x0, x6, w3, uxtw
;;       str     w7, [x6, w3, uxtw]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x7, [x1, #0x28]
;;       ldr     w7, [x7, #8]
;;       add     x8, x6, #4
;;       str     w7, [x8, w3, uxtw]
;;       mov     x8, #0x20
;;       add     x9, x6, #8
;;       str     w8, [x9, w3, uxtw]
;;       mov     w10, #0x2a
;;       str     w10, [x0, #0x18]
;;       mov     x2, x19
;;       str     w2, [x0, #0x10]
;;       mov     w12, #0
;;       str     w12, [x0, #0x14]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x4fc
;;       ├─╼ exception frame offset: SP = FP - 0xb0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0x120
;;  108: mov     w3, #9
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x458
;;  114: ldur    x2, [sp, #0x10]
;;       bl      #0x490
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  11c: .byte   0x1f, 0xc1, 0x00, 0x00
;;       mov     w1, w0
;;       mov     x2, #0x20
;;       adds    x1, x1, x2
;;       cset    x2, hs
;;       tst     w2, #0xff
;;       b.ne    #0x204
;;  138: ldur    x10, [sp, #0x10]
;;       ldr     x2, [x10, #8]
;;       ldr     x3, [x2, #0x28]
;;       cmp     x1, x3
;;       b.hi    #0x1ec
;;  14c: ldr     x1, [x2, #0x20]
;;       add     x1, x1, #0x18
;;       ldr     w0, [x1, w0, uxtw]
;;       stur    w0, [sp, #8]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x48, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x48, patch bytes [1, 1, 0, 148]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x0, [x1, #0x38]
;;       ldr     x2, [x1, #0x48]
;;       ldur    x3, [sp, #0x10]
;;       blr     x0
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  178: ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x4a, patch bytes [250, 0, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4b, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x4b, patch bytes [249, 0, 0, 148]
;;       add     sp, sp, #0x20
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
;;  1b4: mov     w3, #0x4000000
;;  1b8: ldur    x0, [sp, #0x10]
;;  1bc: ldr     x4, [x0, #0x28]
;;  1c0: ldr     w4, [x4, #8]
;;  1c4: mov     w5, #0x20
;;  1c8: mov     w6, #0x10
;;  1cc: ldur    x2, [sp, #0x10]
;;  1d0: bl      #0x3e8
;;  1d4: ldur    x0, [sp, #0x10]
;;  1d8: ldr     x4, [x0, #8]
;;  1dc: ldr     x4, [x4, #0x20]
;;  1e0: add     x0, x4, w2, uxtw
;;  1e4: mov     x3, x2
;;  1e8: b       #0xe8
;;  1ec: mov     w3, #0xfe
;;  1f0: ldur    x2, [sp, #0x10]
;;  1f4: bl      #0x458
;;  1f8: ldur    x2, [sp, #0x10]
;;  1fc: bl      #0x490
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  200: .byte   0x1f, 0xc1, 0x00, 0x00
;;  204: mov     w3, #0xfe
;;  208: ldur    x2, [sp, #0x10]
;;  20c: bl      #0x458
;;  210: ldur    x2, [sp, #0x10]
;;  214: bl      #0x490
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  218: .byte   0x1f, 0xc1, 0x00, 0x00
;;  21c: stur    x2, [sp, #0x10]
;;  220: mov     w3, #0
;;  224: bl      #0x458
;;  228: ldur    x2, [sp, #0x10]
;;  22c: bl      #0x490
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x34, slot at FP-0xb0, locals , stack 
;;  230: .byte   0x1f, 0xc1, 0x00, 0x00
