;;! target = "aarch64"
(module
    (func (result i32)
	(i32.const 0x7fffffff)
	(i32.const -1)
	(i32.sub)
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 e90300aa             	mov	x9, x0
;;      	 ff4300d1             	sub	sp, sp, #0x10
;;      	 fc030091             	mov	x28, sp
;;      	 808300f8             	stur	x0, [x28, #8]
;;      	 810300f8             	stur	x1, [x28]
;;      	 f07b40b2             	orr	x16, xzr, #0x7fffffff
;;      	 e003102a             	mov	w0, w16
;;      	 f07f40b2             	orr	x16, xzr, #0xffffffff
;;      	 0060304b             	sub	w0, w0, w16, uxtx
;;      	 ff430091             	add	sp, sp, #0x10
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
