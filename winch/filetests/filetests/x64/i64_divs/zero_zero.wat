;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0)
	(i64.const 0)
	(i64.div_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8727000000         	ja	0x3f
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c100000000       	mov	rcx, 0
;;      	 48c7c000000000       	mov	rax, 0
;;      	 4883f900             	cmp	rcx, 0
;;      	 0f840d000000         	je	0x41
;;   34:	 4899                 	cqo	
;;      	 48f7f9               	idiv	rcx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3f:	 0f0b                 	ud2	
;;   41:	 0f0b                 	ud2	
