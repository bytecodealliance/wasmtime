;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
      i32.const 15
      i32.popcnt
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
;;       mov     x16, #0xf
;;       mov     w0, w16
;;       fmov    s31, w0
;;       cnt     v31.8b, v31.8b
;;       addv    b31, v31.8b
;;       umov    w0, v31.b[0]
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
