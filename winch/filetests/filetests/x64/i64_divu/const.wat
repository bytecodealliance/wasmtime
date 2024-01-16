;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 20)
	(i64.const 10)
	(i64.div_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c10a000000       	mov	rcx, 0xa
;;      	 48c7c014000000       	mov	rax, 0x14
;;      	 4831d2               	xor	rdx, rdx
;;      	 48f7f1               	div	rcx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
