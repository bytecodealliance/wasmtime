;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "") (param i32) (result i32)
    local.get 0
    i32.const 1
    call 0
    i32.const 1
    call 0
    br_if 0 (;@0;)
    unreachable
  )
)

;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w16, [x28, #4]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, x9
;;       mov     x16, #1
;;       mov     w2, w16
;;       bl      #0
;;   54: add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x14]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       mov     x0, x9
;;       mov     x1, x9
;;       mov     x16, #1
;;       mov     w2, w16
;;       bl      #0
;;   80: ldur    x9, [x28, #0x18]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    w0, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       tst     w1, w1
;;       b.eq    #0xc0
;;       b       #0xb4
;;   b4: add     x28, x28, #4
;;       mov     sp, x28
;;       b       #0xc4
;;   c0: .byte   0x1f, 0xc1, 0x00, 0x00
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
