;;! target = "aarch64"

(module
    (func (result i32)
        (i32.const 1)
     	(i32.const 0)
    	(i32.add)
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff2300d1             	sub	sp, sp, #8
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 890300f8             	stur	x9, [x28]
;;   18:	 300080d2             	mov	x16, #1
;;   1c:	 e003102a             	mov	w0, w16
;;   20:	 00000011             	add	w0, w0, #0
;;   24:	 ff230091             	add	sp, sp, #8
;;   28:	 fc030091             	mov	x28, sp
;;   2c:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   30:	 c0035fd6             	ret	
