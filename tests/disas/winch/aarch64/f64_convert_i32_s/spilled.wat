;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f64)
        i32.const 1
        f64.convert_i32_s
        block
        end
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
;;       mov     x16, #1
;;       mov     w0, w16
;;       scvtf   d0, w0
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    d0, [x28]
;;       ldur    d0, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
