;;! target = "x86_64"

(module
  (func (result i32)
    (i32.const 42)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c02a000000       	mov	rax, 0x2a
;;    b:	 5d                   	pop	rbp
;;    c:	 c3                   	ret	
