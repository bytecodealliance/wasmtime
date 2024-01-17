;;! target = "aarch64"
(module
    (func (result i64)
	(i64.const 0x7fffffffffffffff)
	(i64.const -1)
	(i64.mul)
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 ff2300d1             	sub	sp, sp, #8
;;      	 fc030091             	mov	x28, sp
;;      	 890300f8             	stur	x9, [x28]
;;      	 1000f092             	mov	x16, #0x7fffffffffffffff
;;      	 e00310aa             	mov	x0, x16
;;      	 10008092             	mov	x16, #-1
;;      	 007c109b             	mul	x0, x0, x16
;;      	 ff230091             	add	sp, sp, #8
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
