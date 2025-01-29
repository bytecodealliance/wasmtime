;;! target = "aarch64"
;;! test = "winch"

(module
  (func $main (result i32)
    (local $var i32)
    (call $add (i32.const 20) (i32.const 80))
    (local.set $var (i32.const 2))
    (local.get $var)
    (i32.add))

  (func $add (param i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.add))
)

;; wasm[0]::function[0]::main:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, x9
;;       mov     x16, #0x14
;;       mov     w2, w16
;;       mov     x16, #0x50
;;       mov     w3, w16
;;       bl      #0x80
;;   4c: add     x28, x28, #8
;;       ldur    x9, [x28, #0x10]
;;       mov     x16, #2
;;       mov     w1, w16
;;       stur    w1, [x28, #4]
;;       ldur    w1, [x28, #4]
;;       add     w0, w0, w1, uxtx
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]::add:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       stur    w3, [x28]
;;       ldur    w0, [x28]
;;       ldur    w1, [x28, #4]
;;       add     w1, w1, w0, uxtx
;;       mov     w0, w1
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
