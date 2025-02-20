;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
        (local $foo i64)  
        (local $bar i64)

        (i64.const 1)
        (local.set $foo)

        (i64.const 2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i64.rotr)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       mov     x16, #0
;;       stur    x16, [x28, #8]
;;       stur    x16, [x28]
;;       mov     x16, #1
;;       mov     x0, x16
;;       stur    x0, [x28, #8]
;;       mov     x16, #2
;;       mov     x0, x16
;;       stur    x0, [x28]
;;       ldur    x0, [x28]
;;       ldur    x1, [x28, #8]
;;       ror     x1, x1, x0
;;       mov     x0, x1
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
