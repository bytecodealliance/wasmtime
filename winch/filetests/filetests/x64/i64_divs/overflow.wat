;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;    b:	 48b80000000000000080 	
;; 				movabs	rax, 0x8000000000000000
;;   15:	 4883f900             	cmp	rcx, 0
;;   19:	 0f8407000000         	je	0x26
;;   1f:	 4899                 	cqo	
;;   21:	 48f7f9               	idiv	rcx
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
;;   26:	 0f0b                 	ud2	
