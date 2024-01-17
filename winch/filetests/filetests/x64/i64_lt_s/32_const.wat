;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 2)
        (i64.const 3)
        (i64.lt_s)
    )
)

;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c002000000       	mov	rax, 2
;;      	 4883f803             	cmp	rax, 3
;;      	 b800000000           	mov	eax, 0
;;      	 400f9cc0             	setl	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
