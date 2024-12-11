;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f64)
        i32.const 1
        f64.convert_i32_u
        block
        end
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
;;       mov     x16, #1
;;       mov     w1, w16
;;       ucvtf   d0, w1
;;       sub     sp, sp, #8
;;       mov     x28, sp
;;       stur    d0, [x28]
;;       ldur    d0, [x28]
;;       add     sp, sp, #8
;;       mov     x28, sp
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
