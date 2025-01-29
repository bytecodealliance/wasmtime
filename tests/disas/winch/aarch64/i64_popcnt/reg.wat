;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param i64) (result i64)
      local.get 0
      i64.popcnt
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    x2, [x28]
;;       ldur    x0, [x28]
;;       fmov    d31, x0
;;       cnt     v31.8b, v31.8b
;;       addv    b31, v31.8b
;;       umov    w0, v31.b[0]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
