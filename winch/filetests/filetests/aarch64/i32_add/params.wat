;;! target = "aarch64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.add)
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff2300d1             	sub	sp, sp, #8
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 804300b8             	stur	w0, [x28, #4]
;;   18:	 810300b8             	stur	w1, [x28]
;;   1c:	 800340b8             	ldur	w0, [x28]
;;   20:	 814340b8             	ldur	w1, [x28, #4]
;;   24:	 2160200b             	add	w1, w1, w0, uxtx
;;   28:	 e00301aa             	mov	x0, x1
;;   2c:	 ff230091             	add	sp, sp, #8
;;   30:	 fc030091             	mov	x28, sp
;;   34:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   38:	 c0035fd6             	ret	
