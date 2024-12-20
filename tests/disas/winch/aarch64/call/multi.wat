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
;;       mov     x28, sp
;;       mov     x9, x1
;;       sub     sp, sp, #0x18
;;       mov     x28, sp
;;       stur    x1, [x28, #0x10]
;;       stur    x2, [x28, #8]
;;       stur    x0, [x28]
;;       mov     x16, #2
;;       mov     w0, w16
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       mov     x16, #1
;;       stur    w16, [x28]
;;       ldur    x1, [x28, #4]
;;       ldur    w16, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x1]
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]::start:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x10
;;       mov     x28, sp
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       sub     sp, sp, #0xc
;;       mov     x28, sp
;;       mov     x1, x9
;;       mov     x2, x9
;;       ldur    x0, [x28, #0xc]
;;       bl      #0
;;   a0: add     sp, sp, #0xc
;;       mov     x28, sp
;;       ldur    x9, [x28, #0xc]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
