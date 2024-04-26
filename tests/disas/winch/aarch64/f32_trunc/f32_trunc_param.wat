;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.trunc)
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
;;       stur    w0, [x28, #4]
;;       ldur    w0, [x28, #4]
;;       frintz  s0, s0
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
