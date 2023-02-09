;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 10)
	(i64.const 20)
	(i64.sub)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c00a000000       	mov	rax, 0xa
;;    b:	 4883e814             	sub	rax, 0x14
;;    f:	 5d                   	pop	rbp
;;   10:	 c3                   	ret	
