;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local i32)

        (local.get 0)
        (i32.extend8_s)
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
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    w0, [x28, #4]
;;       sxtb    w0, w0
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
