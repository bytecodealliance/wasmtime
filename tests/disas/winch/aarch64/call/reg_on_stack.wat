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
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x18
;;       mov     x28, sp
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w16, [x28, #4]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x28]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       mov     x0, x9
;;       mov     x1, x9
;;       mov     x16, #1
;;       mov     w2, w16
;;       bl      #0
;;   50: add     sp, sp, #4
;;       mov     x28, sp
;;       ldur    x9, [x28, #0x14]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w0, [x28]
;;       mov     x0, x9
;;       mov     x1, x9
;;       mov     x16, #1
;;       mov     w2, w16
;;       bl      #0
;;   7c: ldur    x9, [x28, #0x18]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w0, [x28]
;;       ldur    w1, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       ldur    w0, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       tst     w1, w1
;;       b.eq    #0xbc
;;       b       #0xb0
;;   b0: add     sp, sp, #4
;;       mov     x28, sp
;;       b       #0xc0
;;   bc: .byte   0x1f, 0xc1, 0x00, 0x00
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
