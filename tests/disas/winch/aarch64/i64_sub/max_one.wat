;;! target = "aarch64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const 1)
	(i64.sub)
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
;;      	 1000f0d2             	mov	x16, #-0x8000000000000000
;;      	 e00310aa             	mov	x0, x16
;;      	 000400d1             	sub	x0, x0, #1
;;      	 ff430091             	add	sp, sp, #0x10
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
