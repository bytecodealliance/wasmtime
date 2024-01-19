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
;;      	 ff4300d1             	sub	sp, sp, #0x10
;;      	 fc030091             	mov	x28, sp
;;      	 100080d2             	mov	x16, #0
;;      	 908300f8             	stur	x16, [x28, #8]
;;      	 890300f8             	stur	x9, [x28]
;;      	 500180d2             	mov	x16, #0xa
;;      	 e003102a             	mov	w0, w16
;;      	 80c300b8             	stur	w0, [x28, #0xc]
;;      	 900280d2             	mov	x16, #0x14
;;      	 e003102a             	mov	w0, w16
;;      	 808300b8             	stur	w0, [x28, #8]
;;      	 808340b8             	ldur	w0, [x28, #8]
;;      	 81c340b8             	ldur	w1, [x28, #0xc]
;;      	 2160204b             	sub	w1, w1, w0, uxtx
;;      	 e003012a             	mov	w0, w1
;;      	 ff430091             	add	sp, sp, #0x10
;;      	 fc030091             	mov	x28, sp
;;      	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;      	 c0035fd6             	ret	
