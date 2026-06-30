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
;;       b.lo    #0x218
;;   48: stur    x2, [sp]
;;       mov     x0, x2
;;       stur    x2, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x35, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x35, patch bytes [67, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x37, patch bytes [65, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3d, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x3d, patch bytes [63, 1, 0, 148]
;;       mov     w1, #0x2a
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3f, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x3f, patch bytes [60, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x40, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x40, patch bytes [59, 1, 0, 148]
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x42, patch bytes [57, 1, 0, 148]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x4c8
;;   88: ldur    x0, [sp, #0x10]
;;       mov     x19, x2
;;       ldr     x0, [x0, #0x20]
;;       ldr     w3, [x0]
;;       mov     w1, w3
;;       add     x1, x1, #0x20
;;       ldr     w2, [x0, #4]
;;       cmp     x1, x2
;;       b.hi    #0x198
;;   ac: ldur    x4, [sp, #0x18]
;;       add     w2, w3, #0x20
;;       str     w2, [x0]
;;       mov     w5, #2
;;       movk    w5, #0x400, lsl #16
;;       ldr     x6, [x4, #0x20]
;;       add     x1, x6, w3, uxtw
;;       str     w5, [x6, w3, uxtw]
;;       ldur    x0, [sp, #0x10]
;;       ldr     x4, [x0, #0x28]
;;       ldr     w4, [x4, #8]
;;       str     w4, [x1, #4]
;;       mov     x5, #0x20
;;       str     w5, [x1, #8]
;;       mov     w7, #0x2a
;;       str     w7, [x1, #0x18]
;;       mov     x2, x19
;;       str     w2, [x1, #0x10]
;;       mov     w9, #0
;;       str     w9, [x1, #0x14]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x500
;;       ├─╼ exception frame offset: SP = FP - 0xb0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0x108
;;       b       #0x1d0
;;  108: mov     w14, w0
;;       mov     x15, #0x20
;;       adds    x13, x14, x15
;;       cset    x15, hs
;;       tst     w15, #0xff
;;       b.ne    #0x200
;;  120: ldur    x2, [sp, #0x18]
;;       ldr     x1, [x2, #0x28]
;;       cmp     x13, x1
;;       b.hi    #0x1e8
;;  130: ldr     x1, [x2, #0x20]
;;       add     x0, x1, w0, uxtw
;;       ldr     w0, [x0, #0x18]
;;       stur    w0, [sp, #8]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x48, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x48, patch bytes [7, 1, 0, 148]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x0, [x1, #0x38]
;;       ldr     x2, [x1, #0x48]
;;       ldur    x3, [sp, #0x10]
;;       blr     x0
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  15c: ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x4a, patch bytes [0, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4b, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x4b, patch bytes [255, 0, 0, 148]
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
;;  198: mov     w3, #2
;;  19c: movk    w3, #0x400, lsl #16
;;  1a0: ldur    x0, [sp, #0x10]
;;  1a4: ldr     x1, [x0, #0x28]
;;  1a8: ldr     w4, [x1, #8]
;;  1ac: mov     w5, #0x20
;;  1b0: mov     w6, #0x10
;;  1b4: ldur    x2, [sp, #0x10]
;;  1b8: bl      #0x3f4
;;  1bc: ldur    x4, [sp, #0x18]
;;  1c0: ldr     x1, [x4, #0x20]
;;  1c4: add     x1, x1, w2, uxtw
;;  1c8: mov     x3, x2
;;  1cc: b       #0xe4
;;  1d0: mov     w3, #9
;;  1d4: ldur    x2, [sp, #0x10]
;;  1d8: bl      #0x45c
;;  1dc: ldur    x2, [sp, #0x10]
;;  1e0: bl      #0x494
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  1e4: udf     #0xc11f
;;  1e8: mov     w3, #0xfe
;;  1ec: ldur    x2, [sp, #0x10]
;;  1f0: bl      #0x45c
;;  1f4: ldur    x2, [sp, #0x10]
;;  1f8: bl      #0x494
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  1fc: udf     #0xc11f
;;  200: mov     w3, #0xfe
;;  204: ldur    x2, [sp, #0x10]
;;  208: bl      #0x45c
;;  20c: ldur    x2, [sp, #0x10]
;;  210: bl      #0x494
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  214: udf     #0xc11f
;;  218: stur    x2, [sp, #0x10]
;;  21c: mov     w3, #0
;;  220: bl      #0x45c
;;  224: ldur    x2, [sp, #0x10]
;;  228: bl      #0x494
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x34, slot at FP-0xb0, locals , stack 
;;  22c: udf     #0xc11f
