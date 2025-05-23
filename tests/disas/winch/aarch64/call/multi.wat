;;! target = "aarch64"
;;! test = "winch"
(module
  (func $multi (result i32 i32)
        i32.const 1
        i32.const 2)

  (func $start
        call $multi
        drop
        drop)
)
;; wasm[0]::function[0]::multi:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x1, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x1c
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x88
;;   2c: mov     x9, x1
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x1, [x28, #0x10]
;;       stur    x2, [x28, #8]
;;       stur    x0, [x28]
;;       mov     x16, #2
;;       mov     w0, w16
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x16, #1
;;       stur    w16, [x28]
;;       ldur    x1, [x28, #4]
;;       ldur    w16, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x1]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   88: .byte   0x1f, 0xc1, 0x00, 0x00
;;
;; wasm[0]::function[1]::start:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x12c
;;   cc: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x1, x9
;;       mov     x2, x9
;;       add     x0, x28, #0xc
;;       bl      #0
;;  100: add     x28, x28, #0xc
;;       mov     sp, x28
;;       ldur    x9, [x28, #0xc]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  12c: .byte   0x1f, 0xc1, 0x00, 0x00
