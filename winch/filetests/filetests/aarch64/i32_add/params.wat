;;! target = "aarch64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.add)
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 ff4300d1             	sub	sp, sp, #0x10
;;      	 fc030091             	mov	x28, sp
;;      	 80c300b8             	stur	w0, [x28, #0xc]
;;      	 818300b8             	stur	w1, [x28, #8]
;;      	 890300f8             	stur	x9, [x28]
;;      	 808340b8             	ldur	w0, [x28, #8]
;;      	 81c340b8             	ldur	w1, [x28, #0xc]
;;      	 2160200b             	add	w1, w1, w0, uxtx
;;      	 e003012a             	mov	w0, w1
;;      	 ff430091             	add	sp, sp, #0x10
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
