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
;;   19:	 0f8502000000         	jne	0x21
;;   1f:	 0f0b                 	ud2	
;;   21:	 4883f9ff             	cmp	rcx, -1
;;   25:	 0f8515000000         	jne	0x40
;;   2b:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   35:	 4c39d8               	cmp	rax, r11
;;   38:	 0f8502000000         	jne	0x40
;;   3e:	 0f0b                 	ud2	
;;   40:	 4899                 	cqo	
;;   42:	 48f7f9               	idiv	rcx
;;   45:	 5d                   	pop	rbp
;;   46:	 c3                   	ret	
