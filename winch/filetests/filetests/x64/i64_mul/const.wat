;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 10)
	(i64.const 20)
	(i64.mul)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c00a000000       	mov	rax, 0xa
;;      	 486bc014             	imul	rax, rax, 0x14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
