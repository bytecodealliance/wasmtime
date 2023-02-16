;;! target = "aarch64"

(module
    (func (result i32)
	(i32.const 10)
	(i32.const 20)
	(i32.add)
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 500180d2             	mov	x16, #0xa
;;   10:	 e003102a             	mov	w0, w16
;;   14:	 00500011             	add	w0, w0, #0x14
;;   18:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   1c:	 c0035fd6             	ret	
