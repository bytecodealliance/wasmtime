;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param i32) (result f64)
        (local.get 0)
        (f64.convert_i32_u)
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
;;       stur    w2, [x28, #4]
;;       ldur    w1, [x28, #4]
;;       ucvtf   d0, w1
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
