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
;;       b.lo    #0x1c0
;;   44: stur    x2, [sp]
;;       mov     x0, x2
;;       stur    x2, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x35, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x35, patch bytes [46, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x37, patch bytes [44, 1, 0, 148]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3d, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x3d, patch bytes [42, 1, 0, 148]
;;       mov     w19, #0x2a
;;       stur    w19, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x3f, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x3f, patch bytes [39, 1, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x40, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x40, patch bytes [38, 1, 0, 148]
;;       stur    w19, [sp, #8]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x42, patch bytes [36, 1, 0, 148]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x468
;;   84: mov     x20, x2
;;       mov     w3, #0x4000000
;;       ldur    x0, [sp, #0x10]
;;       ldr     x0, [x0, #0x28]
;;       ldr     w4, [x0, #8]
;;       mov     w5, #0x28
;;       mov     w6, #8
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x38c
;;   a8: ldur    x0, [sp, #0x10]
;;       ldr     x0, [x0, #8]
;;       ldr     x4, [x0, #0x20]
;;       stur    x0, [sp, #0x20]
;;       add     x0, x4, #0x20
;;       str     w19, [x0, w2, uxtw]
;;       add     x0, x4, #0x18
;;       mov     x1, x20
;;       str     w1, [x0, w2, uxtw]
;;       mov     w0, #0
;;       add     x3, x4, #0x1c
;;       stur    x4, [sp, #0x18]
;;       str     w0, [x3, w2, uxtw]
;;       mov     x3, x2
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x4a0
;;       ├─╼ exception frame offset: SP = FP - 0xc0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0x100
;;   e8: mov     w3, #9
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x3fc
;;   f4: ldur    x2, [sp, #0x10]
;;       bl      #0x434
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x42, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;   fc: .byte   0x1f, 0xc1, 0x00, 0x00
;;       mov     w2, w0
;;       mov     x3, #0x28
;;       adds    x2, x2, x3
;;       cset    x3, hs
;;       tst     w3, #0xff
;;       b.ne    #0x1a8
;;  118: ldur    x1, [sp, #0x20]
;;       ldr     x3, [x1, #0x28]
;;       cmp     x2, x3
;;       b.hi    #0x190
;;  128: ldur    x4, [sp, #0x18]
;;       add     x1, x4, #0x20
;;       ldr     w0, [x1, w0, uxtw]
;;       stur    w0, [sp, #8]
;;       ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x48, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x48, patch bytes [243, 0, 0, 148]
;;       ldur    x1, [sp, #0x10]
;;       ldr     x0, [x1, #0x38]
;;       ldr     x2, [x1, #0x48]
;;       ldur    x3, [sp, #0x10]
;;       blr     x0
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;  154: ldur    x0, [sp, #0x10]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4a, slot at FP-0xc0, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x4a, patch bytes [236, 0, 0, 148]
;;       nop
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x4b, slot at FP-0xc0, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x4b, patch bytes [235, 0, 0, 148]
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
;;  190: mov     w3, #0xfe
;;  194: ldur    x2, [sp, #0x10]
;;  198: bl      #0x3fc
;;  19c: ldur    x2, [sp, #0x10]
;;  1a0: bl      #0x434
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xc0, locals , stack 
;;  1a4: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1a8: mov     w3, #0xfe
;;  1ac: ldur    x2, [sp, #0x10]
;;  1b0: bl      #0x3fc
;;  1b4: ldur    x2, [sp, #0x10]
;;  1b8: bl      #0x434
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x37, slot at FP-0xc0, locals , stack 
;;  1bc: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1c0: stur    x2, [sp, #0x10]
;;  1c4: mov     w3, #0
;;  1c8: bl      #0x3fc
;;  1cc: ldur    x2, [sp, #0x10]
;;  1d0: bl      #0x434
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x34, slot at FP-0xc0, locals , stack 
;;  1d4: .byte   0x1f, 0xc1, 0x00, 0x00
