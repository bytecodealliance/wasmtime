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
        i64.mul
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 ff6300d1             	sub	sp, sp, #0x18
;;      	 fc030091             	mov	x28, sp
;;      	 100080d2             	mov	x16, #0
;;      	 900301f8             	stur	x16, [x28, #0x10]
;;      	 908300f8             	stur	x16, [x28, #8]
;;      	 890300f8             	stur	x9, [x28]
;;      	 500180d2             	mov	x16, #0xa
;;      	 e00310aa             	mov	x0, x16
;;      	 800301f8             	stur	x0, [x28, #0x10]
;;      	 900280d2             	mov	x16, #0x14
;;      	 e00310aa             	mov	x0, x16
;;      	 808300f8             	stur	x0, [x28, #8]
;;      	 808340f8             	ldur	x0, [x28, #8]
;;      	 810341f8             	ldur	x1, [x28, #0x10]
;;      	 217c009b             	mul	x1, x1, x0
;;      	 e00301aa             	mov	x0, x1
;;      	 ff630091             	add	sp, sp, #0x18
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
