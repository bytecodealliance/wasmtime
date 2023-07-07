;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 10)
	(i32.const 20)
	(i32.sub)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b80a000000           	mov	eax, 0xa
;;   11:	 83e814               	sub	eax, 0x14
;;   14:	 4883c408             	add	rsp, 8
;;   18:	 5d                   	pop	rbp
;;   19:	 c3                   	ret	
