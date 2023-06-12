;;! target = "aarch64"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.mul)
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff6300d1             	sub	sp, sp, #0x18
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 800301f8             	stur	x0, [x28, #0x10]
;;   18:	 818300f8             	stur	x1, [x28, #8]
;;   1c:	 890300f8             	stur	x9, [x28]
;;   20:	 808340f8             	ldur	x0, [x28, #8]
;;   24:	 810341f8             	ldur	x1, [x28, #0x10]
;;   28:	 217c009b             	mul	x1, x1, x0
;;   2c:	 e00301aa             	mov	x0, x1
;;   30:	 ff630091             	add	sp, sp, #0x18
;;   34:	 fc030091             	mov	x28, sp
;;   38:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   3c:	 c0035fd6             	ret	
