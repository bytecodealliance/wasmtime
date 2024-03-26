;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.add)
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
;;   20: stur    x2, [x28, #8]
;;   24: stur    x3, [x28]
;;   28: ldur    x0, [x28]
;;   2c: ldur    x1, [x28, #8]
;;   30: add     x1, x1, x0, uxtx
;;   34: mov     x0, x1
;;   38: add     sp, sp, #0x20
;;   3c: mov     x28, sp
;;   40: ldp     x29, x30, [sp], #0x10
;;   44: ret
