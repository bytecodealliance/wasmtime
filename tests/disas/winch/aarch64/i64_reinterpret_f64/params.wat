;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.reinterpret_f64)
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
;;       stur    d0, [x28]
;;       ldur    d0, [x28]
;;       mov     x0, v0.d[0]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
