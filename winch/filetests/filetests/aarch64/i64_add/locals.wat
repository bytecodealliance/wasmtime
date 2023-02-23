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
        i64.add
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff4300d1             	sub	sp, sp, #0x10
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 100080d2             	mov	x16, #0
;;   18:	 908300f8             	stur	x16, [x28, #8]
;;   1c:	 900300f8             	stur	x16, [x28]
;;   20:	 500180d2             	mov	x16, #0xa
;;   24:	 e00310aa             	mov	x0, x16
;;   28:	 808300f8             	stur	x0, [x28, #8]
;;   2c:	 900280d2             	mov	x16, #0x14
;;   30:	 e00310aa             	mov	x0, x16
;;   34:	 800300f8             	stur	x0, [x28]
;;   38:	 800340f8             	ldur	x0, [x28]
;;   3c:	 818340f8             	ldur	x1, [x28, #8]
;;   40:	 2160208b             	add	x1, x1, x0, uxtx
;;   44:	 e00301aa             	mov	x0, x1
;;   48:	 ff430091             	add	sp, sp, #0x10
;;   4c:	 fc030091             	mov	x28, sp
;;   50:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   54:	 c0035fd6             	ret	
