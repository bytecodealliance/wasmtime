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
;;       b.lo    #0x224
;;   44: stur    x2, [sp]
;;       mov     x0, x2
;;       stur    x2, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x35, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x35, patch bytes [71, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x37, patch bytes [69, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3d, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x3d, patch bytes [67, 1, 0, 148]
;;       mov     w1, #0x2a
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3f, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x3f, patch bytes [64, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x40, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x40, patch bytes [63, 1, 0, 148]
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x42, patch bytes [61, 1, 0, 148]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x4cc
;;   84: ldur    x0, [sp, #0x10]
;;       mov     x19, x2
;;       ldr     x4, [x0, #0x20]
;;       ldr     w3, [x4]
;;       mov     w5, w3
;;       add     x5, x5, #0x20
;;       ldr     w6, [x4, #4]
;;       cmp     x5, x6
;;       b.hi    #0x1b8
;;   a8: add     w7, w3, #0x20
;;       str     w7, [x4]
;;       mov     w9, #2
;;       movk    w9, #0x400, lsl #16
;;       ldur    x0, [sp, #0x10]
;;       ldr     x8, [x0, #8]
;;       ldr     x8, [x8, #0x20]
;;       add     x0, x8, w3, uxtw
;;       str     w9, [x8, w3, uxtw]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x9, [x1, #0x28]
;;       ldr     w9, [x9, #8]
;;       add     x10, x8, #4
;;       str     w9, [x10, w3, uxtw]
;;       mov     x10, #0x20
;;       add     x11, x8, #8
;;       str     w10, [x11, w3, uxtw]
;;       mov     w12, #0x2a
;;       str     w12, [x0, #0x18]
;;       mov     x2, x19
;;       str     w2, [x0, #0x10]
;;       mov     w14, #0
;;       str     w14, [x0, #0x14]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x504
;;       ├─╼ exception frame offset: SP = FP - 0xb0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0x124
;;  10c: mov     w3, #9
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x460
;;  118: ldur    x2, [sp, #0x10]
;;       bl      #0x498
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  120: udf     #0xc11f
;;       mov     w1, w0
;;       mov     x2, #0x20
;;       adds    x1, x1, x2
;;       cset    x2, hs
;;       tst     w2, #0xff
;;       b.ne    #0x20c
;;  13c: ldur    x14, [sp, #0x10]
;;       ldr     x2, [x14, #8]
;;       ldr     x3, [x2, #0x28]
;;       cmp     x1, x3
;;       b.hi    #0x1f4
;;  150: ldr     x1, [x2, #0x20]
;;       add     x1, x1, #0x18
;;       ldr     w0, [x1, w0, uxtw]
;;       stur    w0, [sp, #8]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x48, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x48, patch bytes [2, 1, 0, 148]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x0, [x1, #0x38]
;;       ldr     x2, [x1, #0x48]
;;       ldur    x3, [sp, #0x10]
;;       blr     x0
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  17c: ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x4a, patch bytes [251, 0, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4b, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x4b, patch bytes [250, 0, 0, 148]
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
;;  1b8: mov     w3, #2
;;  1bc: movk    w3, #0x400, lsl #16
;;  1c0: ldur    x0, [sp, #0x10]
;;  1c4: ldr     x6, [x0, #0x28]
;;  1c8: ldr     w4, [x6, #8]
;;  1cc: mov     w5, #0x20
;;  1d0: mov     w6, #0x10
;;  1d4: ldur    x2, [sp, #0x10]
;;  1d8: bl      #0x3f0
;;  1dc: ldur    x0, [sp, #0x10]
;;  1e0: ldr     x6, [x0, #8]
;;  1e4: ldr     x6, [x6, #0x20]
;;  1e8: add     x0, x6, w2, uxtw
;;  1ec: mov     x3, x2
;;  1f0: b       #0xec
;;  1f4: mov     w3, #0xfe
;;  1f8: ldur    x2, [sp, #0x10]
;;  1fc: bl      #0x460
;;  200: ldur    x2, [sp, #0x10]
;;  204: bl      #0x498
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  208: udf     #0xc11f
;;  20c: mov     w3, #0xfe
;;  210: ldur    x2, [sp, #0x10]
;;  214: bl      #0x460
;;  218: ldur    x2, [sp, #0x10]
;;  21c: bl      #0x498
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  220: udf     #0xc11f
;;  224: stur    x2, [sp, #0x10]
;;  228: mov     w3, #0
;;  22c: bl      #0x460
;;  230: ldur    x2, [sp, #0x10]
;;  234: bl      #0x498
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x34, slot at FP-0xb0, locals , stack 
;;  238: udf     #0xc11f
