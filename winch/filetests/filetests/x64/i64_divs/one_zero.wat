;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 1)
	(i64.const 0)
	(i64.div_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c100000000       	mov	rcx, 0
;;      	 48c7c001000000       	mov	rax, 1
;;      	 4883f900             	cmp	rcx, 0
;;      	 0f840b000000         	je	0x2f
;;   24:	 4899                 	cqo	
;;      	 48f7f9               	idiv	rcx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   2f:	 0f0b                 	ud2	
