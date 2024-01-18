;;! target = "aarch64"

(module
    (func
        nop
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 ff2300d1             	sub	sp, sp, #8
;;      	 fc030091             	mov	x28, sp
;;      	 890300f8             	stur	x9, [x28]
;;      	 ff230091             	add	sp, sp, #8
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
