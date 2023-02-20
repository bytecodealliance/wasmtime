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
;;   15:	 4883f900             	cmp	rcx, 0
;;   19:	 0f8502000000         	jne	0x21
;;   1f:	 0f0b                 	ud2	
;;   21:	 4883f9ff             	cmp	rcx, -1
;;   25:	 0f850a000000         	jne	0x35
;;   2b:	 b800000000           	mov	eax, 0
;;   30:	 e905000000           	jmp	0x3a
;;   35:	 4899                 	cqo	
;;   37:	 48f7f9               	idiv	rcx
;;   3a:	 4889d0               	mov	rax, rdx
;;   3d:	 5d                   	pop	rbp
;;   3e:	 c3                   	ret	
