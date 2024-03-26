;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo i32)  
        (local $bar i32)

        (i32.const 10)
        (local.set $foo)

        (i32.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i32.add
    )
)
;; wasm[0]::function[0]:
;;    0: stp     x29, x30, [sp, #-0x10]!
;;    4: mov     x29, sp
;;    8: mov     x28, sp
;;    c: mov     x9, x0
;;   10: sub     sp, sp, #0x18
;;   14: mov     x28, sp
;;   18: stur    x0, [x28, #0x10]
;;   1c: stur    x1, [x28, #8]
;;   20: mov     x16, #0
;;   24: stur    x16, [x28]
;;   28: mov     x16, #0xa
;;   2c: mov     w0, w16
;;   30: stur    w0, [x28, #4]
;;   34: mov     x16, #0x14
;;   38: mov     w0, w16
;;   3c: stur    w0, [x28]
;;   40: ldur    w0, [x28]
;;   44: ldur    w1, [x28, #4]
;;   48: add     w1, w1, w0, uxtx
;;   4c: mov     w0, w1
;;   50: add     sp, sp, #0x18
;;   54: mov     x28, sp
;;   58: ldp     x29, x30, [sp], #0x10
;;   5c: ret
