;;! target = "aarch64"
;;! test = "winch"

(module
  (type (;0;) (func (param
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
    i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
  )

    (result i32)
  ))
  (func (export "x") (type 0) local.get 0)
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x28
;;       mov     x28, sp
;;       stur    x0, [x28, #0x20]
;;       stur    x1, [x28, #0x18]
;;       stur    w2, [x28, #0x14]
;;       stur    w3, [x28, #0x10]
;;       stur    w4, [x28, #0xc]
;;       stur    w5, [x28, #8]
;;       stur    w6, [x28, #4]
;;       stur    w7, [x28]
;;       ldur    w0, [x28, #0x14]
;;       add     sp, sp, #0x28
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
