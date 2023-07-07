;;! target = "aarch64"

(module
    (func
        nop
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff2300d1             	sub	sp, sp, #8
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 890300f8             	stur	x9, [x28]
;;   18:	 ff230091             	add	sp, sp, #8
;;   1c:	 fc030091             	mov	x28, sp
;;   20:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   24:	 c0035fd6             	ret	
