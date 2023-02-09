;;! target = "aarch64"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.add)
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff4300d1             	sub	sp, sp, #0x10
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 808300f8             	stur	x0, [x28, #8]
;;   18:	 810300f8             	stur	x1, [x28]
;;   1c:	 800340f8             	ldur	x0, [x28]
;;   20:	 818340f8             	ldur	x1, [x28, #8]
;;   24:	 2160208b             	add	x1, x1, x0, uxtx
;;   28:	 e00301aa             	mov	x0, x1
;;   2c:	 ff430091             	add	sp, sp, #0x10
;;   30:	 fc030091             	mov	x28, sp
;;   34:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   38:	 c0035fd6             	ret	
