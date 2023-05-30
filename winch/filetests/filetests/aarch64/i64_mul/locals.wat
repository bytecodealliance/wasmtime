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
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff6300d1             	sub	sp, sp, #0x18
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 100080d2             	mov	x16, #0
;;   18:	 900301f8             	stur	x16, [x28, #0x10]
;;   1c:	 908300f8             	stur	x16, [x28, #8]
;;   20:	 890300f8             	stur	x9, [x28]
;;   24:	 500180d2             	mov	x16, #0xa
;;   28:	 e00310aa             	mov	x0, x16
;;   2c:	 800301f8             	stur	x0, [x28, #0x10]
;;   30:	 900280d2             	mov	x16, #0x14
;;   34:	 e00310aa             	mov	x0, x16
;;   38:	 808300f8             	stur	x0, [x28, #8]
;;   3c:	 808340f8             	ldur	x0, [x28, #8]
;;   40:	 810341f8             	ldur	x1, [x28, #0x10]
;;   44:	 217c009b             	mul	x1, x1, x0
;;   48:	 e00301aa             	mov	x0, x1
;;   4c:	 ff630091             	add	sp, sp, #0x18
;;   50:	 fc030091             	mov	x28, sp
;;   54:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   58:	 c0035fd6             	ret	
