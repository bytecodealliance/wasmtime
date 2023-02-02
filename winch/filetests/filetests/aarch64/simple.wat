;;! target = "aarch64"

(module
  (func (result i32)
    (i32.const 42)
  )
)

;;    0:	 fd7bbfa9             	stp	x29, x30, [sp, #-0x10]!
;;    4:	 fd030091             	mov	x29, sp
;;    8:	 fc030091             	mov	x28, sp
;;    c:	 500580d2             	mov	x16, #0x2a
;;   10:	 e00310aa             	mov	x0, x16
;;   14:	 fd7bc1a8             	ldp	x29, x30, [sp], #0x10
;;   18:	 c0035fd6             	ret	
