;;! target = "aarch64"

(module
    (func (result i64)
        (local $foo i64)  
        (local $bar i64)

        (i64.const 10)
        (local.set $foo)

        (i64.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i64.sub
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 e90300aa             	mov	x9, x0
;;      	 ff8300d1             	sub	sp, sp, #0x20
;;      	 fc030091             	mov	x28, sp
;;      	 808301f8             	stur	x0, [x28, #0x18]
;;      	 810301f8             	stur	x1, [x28, #0x10]
;;      	 100080d2             	mov	x16, #0
;;      	 908300f8             	stur	x16, [x28, #8]
;;      	 900300f8             	stur	x16, [x28]
;;      	 500180d2             	mov	x16, #0xa
;;      	 e00310aa             	mov	x0, x16
;;      	 808300f8             	stur	x0, [x28, #8]
;;      	 900280d2             	mov	x16, #0x14
;;      	 e00310aa             	mov	x0, x16
;;      	 800300f8             	stur	x0, [x28]
;;      	 800340f8             	ldur	x0, [x28]
;;      	 818340f8             	ldur	x1, [x28, #8]
;;      	 216020cb             	sub	x1, x1, x0, uxtx
;;      	 e00301aa             	mov	x0, x1
;;      	 ff830091             	add	sp, sp, #0x20
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
