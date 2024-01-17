;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 20)
	(i32.const 10)
	(i32.div_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b90a000000           	mov	ecx, 0xa
;;      	 b814000000           	mov	eax, 0x14
;;      	 83f900               	cmp	ecx, 0
;;      	 0f8409000000         	je	0x28
;;   1f:	 99                   	cdq	
;;      	 f7f9                 	idiv	ecx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   28:	 0f0b                 	ud2	
