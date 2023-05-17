;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0)
	(i64.const 0)
	(i64.rem_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c100000000       	mov	rcx, 0
;;   13:	 48c7c000000000       	mov	rax, 0
;;   1a:	 4899                 	cqo	
;;   1c:	 4883f9ff             	cmp	rcx, -1
;;   20:	 0f850a000000         	jne	0x30
;;   26:	 ba00000000           	mov	edx, 0
;;   2b:	 e903000000           	jmp	0x33
;;   30:	 48f7f9               	idiv	rcx
;;   33:	 4889d0               	mov	rax, rdx
;;   36:	 4883c408             	add	rsp, 8
;;   3a:	 5d                   	pop	rbp
;;   3b:	 c3                   	ret	
