;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.sub)
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
;;   20: stur    w2, [x28, #4]
;;   24: stur    w3, [x28]
;;   28: ldur    w0, [x28]
;;   2c: ldur    w1, [x28, #4]
;;   30: sub     w1, w1, w0, uxtx
;;   34: mov     w0, w1
;;   38: add     sp, sp, #0x18
;;   3c: mov     x28, sp
;;   40: ldp     x29, x30, [sp], #0x10
;;   44: ret
