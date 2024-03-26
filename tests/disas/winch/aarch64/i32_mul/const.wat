;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 10)
	(i32.const 20)
	(i32.mul)
    )
)
;; wasm[0]::function[0]:
;;    0: stp     x29, x30, [sp, #-0x10]!
;;    4: mov     x29, sp
;;    8: mov     x28, sp
;;    c: mov     x9, x0
;;   10: sub     sp, sp, #0x10
;;   14: mov     x28, sp
;;   18: stur    x0, [x28, #8]
;;   1c: stur    x1, [x28]
;;   20: mov     x16, #0xa
;;   24: mov     w0, w16
;;   28: mov     x16, #0x14
;;   2c: mul     w0, w0, w16
;;   30: add     sp, sp, #0x10
;;   34: mov     x28, sp
;;   38: ldp     x29, x30, [sp], #0x10
;;   3c: ret
