;;! target = "aarch64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.mul)
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 e90300aa             	mov	x9, x0
;;      	 ff6300d1             	sub	sp, sp, #0x18
;;      	 fc030091             	mov	x28, sp
;;      	 800301f8             	stur	x0, [x28, #0x10]
;;      	 818300f8             	stur	x1, [x28, #8]
;;      	 824300b8             	stur	w2, [x28, #4]
;;      	 830300b8             	stur	w3, [x28]
;;      	 800340b8             	ldur	w0, [x28]
;;      	 814340b8             	ldur	w1, [x28, #4]
;;      	 217c001b             	mul	w1, w1, w0
;;      	 e003012a             	mov	w0, w1
;;      	 ff630091             	add	sp, sp, #0x18
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
