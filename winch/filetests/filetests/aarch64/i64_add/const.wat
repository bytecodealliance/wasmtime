;;! target = "aarch64"

(module
    (func (result i64)
	(i64.const 10)
	(i64.const 20)
	(i64.add)
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 500180d2             	mov	x16, #0xa
;;   10:	 e00310aa             	mov	x0, x16
;;   14:	 00500091             	add	x0, x0, #0x14
;;   18:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   1c:	 c0035fd6             	ret	
