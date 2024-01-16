;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 7)
	(i64.const 5)
	(i64.rem_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c105000000       	mov	rcx, 5
;;      	 48c7c007000000       	mov	rax, 7
;;      	 4831d2               	xor	rdx, rdx
;;      	 48f7f1               	div	rcx
;;      	 4889d0               	mov	rax, rdx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
