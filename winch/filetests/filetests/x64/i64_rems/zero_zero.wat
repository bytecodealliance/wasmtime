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
;;    4:	 48c7c100000000       	mov	rcx, 0
;;    b:	 48c7c000000000       	mov	rax, 0
;;   12:	 4899                 	cqo	
;;   14:	 4883f9ff             	cmp	rcx, -1
;;   18:	 0f850a000000         	jne	0x28
;;   1e:	 ba00000000           	mov	edx, 0
;;   23:	 e903000000           	jmp	0x2b
;;   28:	 48f7f9               	idiv	rcx
;;   2b:	 4889d0               	mov	rax, rdx
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	
