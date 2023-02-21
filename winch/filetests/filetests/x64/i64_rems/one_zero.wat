;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 1)
	(i64.const 0)
	(i64.rem_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c100000000       	mov	rcx, 0
;;    b:	 48c7c001000000       	mov	rax, 1
;;   12:	 4883f900             	cmp	rcx, 0
;;   16:	 0f8502000000         	jne	0x1e
;;   1c:	 0f0b                 	ud2	
;;   1e:	 4883f9ff             	cmp	rcx, -1
;;   22:	 0f850a000000         	jne	0x32
;;   28:	 b800000000           	mov	eax, 0
;;   2d:	 e905000000           	jmp	0x37
;;   32:	 4899                 	cqo	
;;   34:	 48f7f9               	idiv	rcx
;;   37:	 4889d0               	mov	rax, rdx
;;   3a:	 5d                   	pop	rbp
;;   3b:	 c3                   	ret	
