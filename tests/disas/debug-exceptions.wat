;;! target = "aarch64"
;;! test = "compile"
;;! flags = ["-Wexceptions=yes", "-Wgc=yes", "-Ddebug-instrumentation=yes"]

(module
  (tag $t (param i32))
  (import "" "host" (func))
  (func (export "main")
    (block $b (result i32)
      (try_table (catch $t $b)
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
;;       b.lo    #0x10c
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
;;       mov     w24, #0x2a
;;       stur    w24, [sp, #8]
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x318
;;   5c: mov     x21, x2
;;       mov     w3, #0x4000000
;;       mov     w4, #2
;;       mov     w5, #0x28
;;       mov     w6, #8
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x2a4
;;   78: ldur    x4, [sp, #0x10]
;;       ldr     x9, [x4, #8]
;;       ldr     x13, [x9, #0x18]
;;       add     x9, x13, #0x20
;;       str     w24, [x9, w2, uxtw]
;;       add     x10, x13, #0x18
;;       mov     x12, x21
;;       str     w12, [x10, w2, uxtw]
;;       mov     w11, #0
;;       add     x12, x13, #0x1c
;;       stur    x13, [sp, #0x18]
;;       str     w11, [x12, w2, uxtw]
;;       mov     x3, x2
;;       ldur    x2, [sp, #0x10]
;;       bl      #0x350
;;       ├─╼ exception frame offset: SP = FP - 0xb0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x10], handler=0xb8
;;   b4: .byte   0x1f, 0xc1, 0x00, 0x00
;;       ldur    x13, [sp, #0x18]
;;       add     x13, x13, #0x20
;;       ldr     w15, [x13, w0, uxtw]
;;       stur    w15, [sp, #8]
;;       ldur    x2, [sp, #0x10]
;;       ldr     x0, [x2, #0x30]
;;       ldr     x2, [x2, #0x40]
;;       ldur    x3, [sp, #0x10]
;;       blr     x0
;;       ╰─╼ debug frame state: func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 69, slot at FP-0xb0, locals , stack I32 @ slot+0x8
;;   dc: add     sp, sp, #0x20
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
;;  10c: .byte   0x1f, 0xc1, 0x00, 0x00
