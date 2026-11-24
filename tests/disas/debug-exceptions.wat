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
;;       ldur    x16, [x2, #8]
;;       ldur    x16, [x16, #0x10]
;;       add     x16, x16, #0xd0
;;       cmp     sp, x16
;;       b.hs    #0x18
;;       b       #0x4ac
;;   18: stp     x29, x30, [sp, #-0x10]!
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
;;       stur    x2, [sp]
;;       mov     x0, x2
;;       stur    x2, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x35, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x35, patch bytes [83, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x37, patch bytes [81, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3d, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x3d, patch bytes [79, 1, 0, 148]
;;       mov     w1, #0x2a
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3f, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x3f, patch bytes [76, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x40, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x40, patch bytes [75, 1, 0, 148]
;;       stur    w1, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x42, patch bytes [73, 1, 0, 148]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x508
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;   88: ldur    x0, [sp, #0x10]
;;       ldr     x0, [x0, #0x20]
;;       ldr     w3, [x0]
;;       mov     w1, w3
;;       add     x1, x1, #0x20
;;       ldr     w4, [x0, #4]
;;       cmp     x1, x4
;;       b.hi    #0x190
;;   a8: add     w1, w3, #0x20
;;       str     w1, [x0]
;;       mov     w1, #2
;;       movk    w1, #0x400, lsl #16
;;       ldur    x0, [sp, #0x10]
;;       ldr     x0, [x0, #8]
;;       ldr     x4, [x0, #0x18]
;;       add     x0, x4, w3, uxtw
;;       str     w1, [x4, w3, uxtw]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x1, [x1, #0x28]
;;       ldr     w1, [x1, #8]
;;       str     w1, [x0, #4]
;;       mov     x1, #0x20
;;       str     w1, [x0, #8]
;;       mov     w4, #0x2a
;;       str     w4, [x0, #0x18]
;;       str     w2, [x0, #0x10]
;;       mov     w5, #0
;;       str     w5, [x0, #0x14]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x540
;;       ├─╼ exception frame offset: SP = FP - 0xb0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0x104
;;       b       #0x1d4
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  104: mov     w10, w0
;;       mov     x11, #0x20
;;       adds    x10, x10, x11
;;       b.hs    #0x204
;;  114: ldur    x1, [sp, #0x10]
;;       ldr     x11, [x1, #8]
;;       ldr     x12, [x11, #0x20]
;;       cmp     x10, x12
;;       b.hi    #0x1ec
;;  128: ldr     x13, [x11, #0x18]
;;       add     x13, x13, w0, uxtw
;;       ldr     w13, [x13, #0x18]
;;       stur    w13, [sp, #8]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x48, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x48, patch bytes [25, 1, 0, 148]
;;       ldur    x11, [sp, #0x10]
;;       ldr     x0, [x11, #0x38]
;;       ldr     x2, [x11, #0x48]
;;       ldur    x3, [sp, #0x10]
;;       blr     x0
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  154: ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x4a, patch bytes [18, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4b, slot at FP-0xb0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x4b, patch bytes [17, 1, 0, 148]
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
;;  190: mov     x19, x2
;;  194: mov     w3, #2
;;  198: movk    w3, #0x400, lsl #16
;;  19c: ldur    x0, [sp, #0x10]
;;  1a0: ldr     x0, [x0, #0x28]
;;  1a4: ldr     w4, [x0, #8]
;;  1a8: mov     w5, #0x20
;;  1ac: mov     w6, #0x10
;;  1b0: ldur    x2, [sp, #0x10]
;;  1b4: bl      #0x3d8
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  1b8: ldur    x0, [sp, #0x10]
;;  1bc: ldr     x0, [x0, #8]
;;  1c0: ldr     x0, [x0, #0x18]
;;  1c4: add     x0, x0, w2, uxtw
;;  1c8: mov     x3, x2
;;  1cc: mov     x2, x19
;;  1d0: b       #0xe4
;;  1d4: mov     w3, #9
;;  1d8: ldur    x2, [sp, #0x10]
;;  1dc: bl      #0x440
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  1e0: ldur    x2, [sp, #0x10]
;;  1e4: bl      #0x478
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;  1e8: udf     #0xc11f
;;  1ec: mov     w3, #0xfe
;;  1f0: ldur    x2, [sp, #0x10]
;;  1f4: bl      #0x440
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  1f8: ldur    x2, [sp, #0x10]
;;  1fc: bl      #0x478
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  200: udf     #0xc11f
;;  204: mov     w3, #0xfe
;;  208: ldur    x2, [sp, #0x10]
;;  20c: bl      #0x440
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  210: ldur    x2, [sp, #0x10]
;;  214: bl      #0x478
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xb0, locals , stack 
;;  218: udf     #0xc11f
