;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo i32)  
        (i32.const 10)
        (local.tee $foo)
        i32.ctz
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
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       mov     x16, #0xa
;;       mov     w0, w16
;;       stur    w0, [x28, #4]
;;       rbit    w16, w0
;;       clz     w0, w16
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
