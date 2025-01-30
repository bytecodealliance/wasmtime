;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
        (i64.const 9223372036854775806)
        (i64.const 9223372036854775807)
        (i64.or)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       orr     x16, xzr, #0x7ffffffffffffffe
;;       mov     x0, x16
;;       orr     x0, x0, #0x7fffffffffffffff
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
