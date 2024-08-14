;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param i64) (result i64)
        (local.get 0)
        (i64.extend8_s)
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
;;       stur    x2, [x28]
;;       ldur    x0, [x28]
;;       sxtb    x0, w0
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
