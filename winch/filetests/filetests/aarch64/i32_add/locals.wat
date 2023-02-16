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
        i32.add
    )
)
;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 ff2300d1             	sub	sp, sp, #8
;;   10:	 fc030091             	mov	x28, sp
;;   14:	 100080d2             	mov	x16, #0
;;   18:	 900300f8             	stur	x16, [x28]
;;   1c:	 500180d2             	mov	x16, #0xa
;;   20:	 e003102a             	mov	w0, w16
;;   24:	 804300b8             	stur	w0, [x28, #4]
;;   28:	 900280d2             	mov	x16, #0x14
;;   2c:	 e003102a             	mov	w0, w16
;;   30:	 800300b8             	stur	w0, [x28]
;;   34:	 800340b8             	ldur	w0, [x28]
;;   38:	 814340b8             	ldur	w1, [x28, #4]
;;   3c:	 2160200b             	add	w1, w1, w0, uxtx
;;   40:	 e00301aa             	mov	x0, x1
;;   44:	 ff230091             	add	sp, sp, #8
;;   48:	 fc030091             	mov	x28, sp
;;   4c:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   50:	 c0035fd6             	ret	
