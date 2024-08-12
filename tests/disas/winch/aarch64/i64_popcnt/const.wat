;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
      i64.const 3
      i64.popcnt
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x10
;;       mov     x28, sp
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #3
;;       mov     x0, x16
;;       fmov    d0, x0
;;       cnt     v0.8b, v0.8b
;;       addv    b0, v0.8b
;;       umov    w0, v0.b[0]
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
