;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 1)
        (i64.eqz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c001000000       	mov	rax, 1
;;      	 4883f800             	cmp	rax, 0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
