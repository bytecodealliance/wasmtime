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
        i32.sub
    )
)
;;      	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;      	 fd030091             	mov	x29, sp
;;      	 fc030091             	mov	x28, sp
;;      	 e90300aa             	mov	x9, x0
;;      	 ff6300d1             	sub	sp, sp, #0x18
;;      	 fc030091             	mov	x28, sp
;;      	 800301f8             	stur	x0, [x28, #0x10]
;;      	 818300f8             	stur	x1, [x28, #8]
;;      	 100080d2             	mov	x16, #0
;;      	 900300f8             	stur	x16, [x28]
;;      	 500180d2             	mov	x16, #0xa
;;      	 e003102a             	mov	w0, w16
;;      	 804300b8             	stur	w0, [x28, #4]
;;      	 900280d2             	mov	x16, #0x14
;;      	 e003102a             	mov	w0, w16
;;      	 800300b8             	stur	w0, [x28]
;;      	 800340b8             	ldur	w0, [x28]
;;      	 814340b8             	ldur	w1, [x28, #4]
;;      	 2160204b             	sub	w1, w1, w0, uxtx
;;      	 e003012a             	mov	w0, w1
;;      	 ff630091             	add	sp, sp, #0x18
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
