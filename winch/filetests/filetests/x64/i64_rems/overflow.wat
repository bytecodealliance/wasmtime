;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.rem_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;      	 48b80000000000000080 	
;; 				movabs	rax, 0x8000000000000000
;;      	 4899                 	cqo	
;;      	 4883f9ff             	cmp	rcx, -1
;;      	 0f850a000000         	jne	0x33
;;   29:	 ba00000000           	mov	edx, 0
;;      	 e903000000           	jmp	0x36
;;   33:	 48f7f9               	idiv	rcx
;;      	 4889d0               	mov	rax, rdx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
