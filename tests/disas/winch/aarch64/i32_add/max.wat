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
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x10
;;       mov     x28, sp
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       orr     x16, xzr, #0x7fffffff
;;       mov     w0, w16
;;       add     w0, w0, #1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
