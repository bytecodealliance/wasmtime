;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.rem_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;    b:	 48b80000000000000080 	
;; 				movabs	rax, 0x8000000000000000
;;   15:	 4899                 	cqo	
;;   17:	 4883f9ff             	cmp	rcx, -1
;;   1b:	 0f850a000000         	jne	0x2b
;;   21:	 ba00000000           	mov	edx, 0
;;   26:	 e903000000           	jmp	0x2e
;;   2b:	 48f7f9               	idiv	rcx
;;   2e:	 4889d0               	mov	rax, rdx
;;   31:	 5d                   	pop	rbp
;;   32:	 c3                   	ret	
