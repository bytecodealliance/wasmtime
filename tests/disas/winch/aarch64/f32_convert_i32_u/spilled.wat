;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f32)
        i32.const 1
        f32.convert_i32_u
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
;;       ucvtf   s0, w1
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    s0, [x28]
;;       ldur    s0, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
