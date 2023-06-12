;;! target = "aarch64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.mul)
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff4300d1             	sub	sp, sp, #0x10
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 80c300b8             	stur	w0, [x28, #0xc]
;;   18:	 818300b8             	stur	w1, [x28, #8]
;;   1c:	 890300f8             	stur	x9, [x28]
;;   20:	 808340b8             	ldur	w0, [x28, #8]
;;   24:	 81c340b8             	ldur	w1, [x28, #0xc]
;;   28:	 217c001b             	mul	w1, w1, w0
;;   2c:	 e00301aa             	mov	x0, x1
;;   30:	 ff430091             	add	sp, sp, #0x10
;;   34:	 fc030091             	mov	x28, sp
;;   38:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   3c:	 c0035fd6             	ret	
