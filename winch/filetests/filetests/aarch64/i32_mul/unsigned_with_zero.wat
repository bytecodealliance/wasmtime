;;! target = "aarch64"

(module
    (func (result i32)
        (i32.const 1)
     	(i32.const 0)
    	(i32.mul)
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 ff2300d1             	sub	sp, sp, #8
;;      	 fc030091             	mov	x28, sp
;;      	 890300f8             	stur	x9, [x28]
;;      	 300080d2             	mov	x16, #1
;;      	 e003102a             	mov	w0, w16
;;      	 100080d2             	mov	x16, #0
;;      	 007c101b             	mul	w0, w0, w16
;;      	 ff230091             	add	sp, sp, #8
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
