;;! target = "aarch64"
;;! test = "winch"
(module
    (func (result i32)
	(i32.const 0x7fffffff)
	(i32.const 1)
	(i32.add)
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
;;   20: orr     x16, xzr, #0x7fffffff
;;   24: mov     w0, w16
;;   28: add     w0, w0, #1
;;   2c: add     sp, sp, #0x10
;;   30: mov     x28, sp
;;   34: ldp     x29, x30, [sp], #0x10
;;   38: ret
