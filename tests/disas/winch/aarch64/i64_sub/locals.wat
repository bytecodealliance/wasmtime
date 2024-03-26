;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
        (local $foo i64)  
        (local $bar i64)

        (i64.const 10)
        (local.set $foo)

        (i64.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i64.sub
    )
)
;; wasm[0]::function[0]:
;;    0: stp     x29, x30, [sp, #-0x10]!
;;    4: mov     x29, sp
;;    8: mov     x28, sp
;;    c: mov     x9, x0
;;   10: sub     sp, sp, #0x20
;;   14: mov     x28, sp
;;   18: stur    x0, [x28, #0x18]
;;   1c: stur    x1, [x28, #0x10]
;;   20: mov     x16, #0
;;   24: stur    x16, [x28, #8]
;;   28: stur    x16, [x28]
;;   2c: mov     x16, #0xa
;;   30: mov     x0, x16
;;   34: stur    x0, [x28, #8]
;;   38: mov     x16, #0x14
;;   3c: mov     x0, x16
;;   40: stur    x0, [x28]
;;   44: ldur    x0, [x28]
;;   48: ldur    x1, [x28, #8]
;;   4c: sub     x1, x1, x0, uxtx
;;   50: mov     x0, x1
;;   54: add     sp, sp, #0x20
;;   58: mov     x28, sp
;;   5c: ldp     x29, x30, [sp], #0x10
;;   60: ret
