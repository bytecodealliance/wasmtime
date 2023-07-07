;;! target = "aarch64"

(module
    (func (result i32)
        (local $foo i32)  
        (local $bar i32)

        (i32.const 10)
        (local.set $foo)

        (i32.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i32.mul
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff4300d1             	sub	sp, sp, #0x10
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 100080d2             	mov	x16, #0
;;   18:	 908300f8             	stur	x16, [x28, #8]
;;   1c:	 890300f8             	stur	x9, [x28]
;;   20:	 500180d2             	mov	x16, #0xa
;;   24:	 e003102a             	mov	w0, w16
;;   28:	 80c300b8             	stur	w0, [x28, #0xc]
;;   2c:	 900280d2             	mov	x16, #0x14
;;   30:	 e003102a             	mov	w0, w16
;;   34:	 808300b8             	stur	w0, [x28, #8]
;;   38:	 808340b8             	ldur	w0, [x28, #8]
;;   3c:	 81c340b8             	ldur	w1, [x28, #0xc]
;;   40:	 217c001b             	mul	w1, w1, w0
;;   44:	 e00301aa             	mov	x0, x1
;;   48:	 ff430091             	add	sp, sp, #0x10
;;   4c:	 fc030091             	mov	x28, sp
;;   50:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   54:	 c0035fd6             	ret	
