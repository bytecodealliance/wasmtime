;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 1)
	(i64.const 0)
	(i64.rem_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f873d000000         	ja	0x5b
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 48c7c100000000       	mov	rcx, 0
;;      	 48c7c001000000       	mov	rax, 1
;;      	 4899                 	cqo	
;;      	 4883f9ff             	cmp	rcx, -1
;;      	 0f850a000000         	jne	0x4f
;;   45:	 ba00000000           	mov	edx, 0
;;      	 e903000000           	jmp	0x52
;;   4f:	 48f7f9               	idiv	rcx
;;      	 4889d0               	mov	rax, rdx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5b:	 0f0b                 	ud2	
