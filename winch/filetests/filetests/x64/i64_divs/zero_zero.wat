;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0)
	(i64.const 0)
	(i64.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c100000000       	mov	rcx, 0
;;    b:	 48c7c000000000       	mov	rax, 0
;;   12:	 4883f900             	cmp	rcx, 0
;;   16:	 0f8502000000         	jne	0x1e
;;   1c:	 0f0b                 	ud2	
;;   1e:	 4883f9ff             	cmp	rcx, -1
;;   22:	 0f8515000000         	jne	0x3d
;;   28:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   32:	 4c39d8               	cmp	rax, r11
;;   35:	 0f8502000000         	jne	0x3d
;;   3b:	 0f0b                 	ud2	
;;   3d:	 4899                 	cqo	
;;   3f:	 48f7f9               	idiv	rcx
;;   42:	 5d                   	pop	rbp
;;   43:	 c3                   	ret	
