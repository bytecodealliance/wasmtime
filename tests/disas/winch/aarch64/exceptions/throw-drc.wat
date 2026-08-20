;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=drc"

;; `throw` builds an exception that escapes to the host, while `try_table`
;; still compiles as a plain block.
(module
  (tag $e (param i32))
  (func (result i32)
    (block $h (result i32)
      (try_table (result i32) (catch $e $h)
        (throw $e (i32.const 42))))))
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x120
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x0, x9
;;       bl      #0x25c
;;   48: ldur    x9, [x28, #8]
;;       ldur    x1, [x9, #0x28]
;;       ldur    w1, [x1, #8]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, #0x4000000
;;       ldur    w2, [x28, #8]
;;       mov     x3, #0x28
;;       mov     x4, #8
;;       bl      #0x20c
;;   8c: add     x28, x28, #8
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0xc]
;;       ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x2, #0x18]
;;       mov     x16, #0
;;       stur    w16, [x2, #0x1c]
;;       mov     x16, #0x2a
;;       stur    w16, [x2, #0x20]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #0xc]
;;       bl      #0x28c
;;   f4: add     x28, x28, #0xc
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #8]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  120: udf     #0xc11f
