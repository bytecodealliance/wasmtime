;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo i32)
        (local $bar i32)

        (i32.const -2)
        (local.set $foo)
        (i32.const -3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i32.le_s)
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
;;       orr     x16, xzr, #0xfffffffe
;;       mov     w0, w16
;;       stur    w0, [x28, #4]
;;       mov     w16, #-3
;;       mov     w0, w16
;;       stur    w0, [x28]
;;       ldur    w0, [x28]
;;       ldur    w1, [x28, #4]
;;       cmp     w1, w0, uxtx
;;       cset    x1, le
;;       mov     w0, w1
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
