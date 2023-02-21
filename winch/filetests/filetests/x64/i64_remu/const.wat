;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 7)
	(i64.const 5)
	(i64.rem_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c105000000       	mov	rcx, 5
;;    b:	 48c7c007000000       	mov	rax, 7
;;   12:	 4883f900             	cmp	rcx, 0
;;   16:	 0f8502000000         	jne	0x1e
;;   1c:	 0f0b                 	ud2	
;;   1e:	 ba00000000           	mov	edx, 0
;;   23:	 48f7f1               	div	rcx
;;   26:	 4889d0               	mov	rax, rdx
;;   29:	 5d                   	pop	rbp
;;   2a:	 c3                   	ret	
