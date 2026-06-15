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
;;       ldr     x1, [x0, #0x18]
;;       stur    x0, [sp, #0x18]
;;       mov     x0, sp
;;       cmp     x0, x1
;;       b.lo    #0x21c
;;   48: stur    x2, [sp]
;;       mov     x0, x2
;;       stur    x2, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x35, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x35, patch bytes [61, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x37, patch bytes [59, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3d, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x3d, patch bytes [57, 1, 0, 148]
;;       mov     w1, #0x2a
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3f, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x3f, patch bytes [54, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x40, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x40, patch bytes [53, 1, 0, 148]
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x42, patch bytes [51, 1, 0, 148]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x4b0
;;   88: ldur    x0, [sp, #0x10]
;;       mov     x19, x2
;;       ldr     x1, [x0, #0x20]
;;       ldr     w3, [x1]
;;       mov     w2, w3
;;       add     x2, x2, #0x20
;;       ldr     w4, [x1, #4]
;;       cmp     x2, x4
;;       b.hi    #0x1b4
;;   ac: ldur    x0, [sp, #0x18]
;;       add     w4, w3, #0x20
;;       str     w4, [x1]
;;       mov     w6, #2
;;       movk    w6, #0x400, lsl #16
;;       ldr     x5, [x0, #0x20]
;;       add     x1, x5, w3, uxtw
;;       str     w6, [x5, w3, uxtw]
;;       ldur    x0, [sp, #0x10]
;;       ldr     x6, [x0, #0x28]
;;       ldr     w6, [x6, #8]
;;       add     x7, x5, #4
;;       str     w6, [x7, w3, uxtw]
;;       mov     x7, #0x20
;;       add     x8, x5, #8
;;       str     w7, [x8, w3, uxtw]
;;       mov     w9, #0x2a
;;       str     w9, [x1, #0x18]
;;       mov     x2, x19
;;       str     w2, [x1, #0x10]
;;       mov     w11, #0
;;       str     w11, [x1, #0x14]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x4e8
;;       ├─╼ exception frame offset: SP = FP - 0xb0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0x124
;;  10c: mov     w3, #9
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x444
;;  118: ldur    x2, [sp, #0x10]
;;       bl      #0x47c
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  120: udf     #0xc11f
;;       mov     w1, w0
;;       mov     x2, #0x20
;;       adds    x15, x1, x2
;;       cset    x1, hs
;;       tst     w1, #0xff
;;       b.ne    #0x204
;;  13c: ldur    x2, [sp, #0x18]
;;       ldr     x1, [x2, #0x28]
;;       cmp     x15, x1
;;       b.hi    #0x1ec
;;  14c: ldr     x1, [x2, #0x20]
;;       add     x1, x1, #0x18
;;       ldr     w0, [x1, w0, uxtw]
;;       stur    w0, [sp, #8]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x48, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x48, patch bytes [250, 0, 0, 148]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x0, [x1, #0x38]
;;       ldr     x2, [x1, #0x48]
;;       ldur    x3, [sp, #0x10]
;;       blr     x0
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  178: ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x4a, patch bytes [243, 0, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4b, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x4b, patch bytes [242, 0, 0, 148]
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
;;  1b4: mov     w3, #2
;;  1b8: movk    w3, #0x400, lsl #16
;;  1bc: ldur    x0, [sp, #0x10]
;;  1c0: ldr     x4, [x0, #0x28]
;;  1c4: ldr     w4, [x4, #8]
;;  1c8: mov     w5, #0x20
;;  1cc: mov     w6, #0x10
;;  1d0: ldur    x2, [sp, #0x10]
;;  1d4: bl      #0x3dc
;;  1d8: ldur    x0, [sp, #0x18]
;;  1dc: ldr     x3, [x0, #0x20]
;;  1e0: add     x1, x3, w2, uxtw
;;  1e4: mov     x3, x2
;;  1e8: b       #0xec
;;  1ec: mov     w3, #0xfe
;;  1f0: ldur    x2, [sp, #0x10]
;;  1f4: bl      #0x444
;;  1f8: ldur    x2, [sp, #0x10]
;;  1fc: bl      #0x47c
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  200: udf     #0xc11f
;;  204: mov     w3, #0xfe
;;  208: ldur    x2, [sp, #0x10]
;;  20c: bl      #0x444
;;  210: ldur    x2, [sp, #0x10]
;;  214: bl      #0x47c
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  218: udf     #0xc11f
;;  21c: stur    x2, [sp, #0x10]
;;  220: mov     w3, #0
;;  224: bl      #0x444
;;  228: ldur    x2, [sp, #0x10]
;;  22c: bl      #0x47c
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x34, slot at FP-0xb0, locals , stack 
;;  230: udf     #0xc11f
