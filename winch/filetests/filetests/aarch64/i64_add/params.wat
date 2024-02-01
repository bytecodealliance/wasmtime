;;! target = "aarch64"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.add)
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 ff6300d1             	sub	sp, sp, #0x18
;;      	 fc030091             	mov	x28, sp
;;      	 800301f8             	stur	x0, [x28, #0x10]
;;      	 818300f8             	stur	x1, [x28, #8]
;;      	 890300f8             	stur	x9, [x28]
;;      	 808340f8             	ldur	x0, [x28, #8]
;;      	 810341f8             	ldur	x1, [x28, #0x10]
;;      	 2160208b             	add	x1, x1, x0, uxtx
;;      	 e00301aa             	mov	x0, x1
;;      	 ff630091             	add	sp, sp, #0x18
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
