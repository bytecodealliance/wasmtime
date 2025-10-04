;;! target = "aarch64"
;;! test = "compile"
;;! flags = ["-Wexceptions=yes", "-Wgc=yes", "-Ddebug-instrumentation=yes"]

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
;;       ldur    x16, [x2, #8]
;;       ldur    x16, [x16, #0x10]
;;       add     x16, x16, #0xc0
;;       cmp     sp, x16
;;       b.lo    #0x110
;;   1c: stp     x27, x28, [sp, #-0x10]!
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
;;       stur    x2, [sp, #0x10]
;;       mov     w27, #0x2a
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 61, slot at FP-0xb0, locals , stack 
;;       stur    w27, [sp, #8]
;;       stur    w27, [sp, #8]
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 64, slot at FP-0xb0, locals , stack 
;;       ldur    x2, [sp, #0x10]
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 66, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       bl      #0x31c
;;   60: mov     x21, x2
;;       mov     w3, #0x4000000
;;       mov     w4, #2
;;       mov     w5, #0x28
;;       mov     w6, #8
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x2a8
;;   7c: ldur    x11, [sp, #0x10]
;;       ldr     x0, [x11, #8]
;;       ldr     x5, [x0, #0x18]
;;       add     x0, x5, #0x20
;;       str     w27, [x0, w2, uxtw]
;;       add     x3, x5, #0x18
;;       mov     x4, x21
;;       str     w4, [x3, w2, uxtw]
;;       mov     w3, #0
;;       add     x4, x5, #0x1c
;;       stur    x5, [sp, #0x18]
;;       str     w3, [x4, w2, uxtw]
;;       mov     x3, x2
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x354
;;       ├─╼ exception frame offset: SP = FP - 0xb0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0xbc
;;   b8: .byte   0x1f, 0xc1, 0x00, 0x00
;;       ldur    x5, [sp, #0x18]
;;       add     x4, x5, #0x20
;;       ldr     w6, [x4, w0, uxtw]
;;       stur    w6, [sp, #8]
;;       ldur    x2, [sp, #0x10]
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 72, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;       ldr     x8, [x2, #0x30]
;;       ldr     x2, [x2, #0x40]
;;       ldur    x3, [sp, #0x10]
;;       blr     x8
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 72, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;   e0: add     sp, sp, #0x20
;;       ╰─╼ debug frame state (before next inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 75, slot at FP-0xb0, locals , stack 
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
;;  110: .byte   0x1f, 0xc1, 0x00, 0x00
