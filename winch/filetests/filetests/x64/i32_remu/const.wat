;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 7)
	(i32.const 5)
	(i32.rem_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 b905000000           	mov	ecx, 5
;;    9:	 b807000000           	mov	eax, 7
;;    e:	 83f900               	cmp	ecx, 0
;;   11:	 0f8502000000         	jne	0x19
;;   17:	 0f0b                 	ud2	
;;   19:	 ba00000000           	mov	edx, 0
;;   1e:	 f7f1                 	div	ecx
;;   20:	 4889d0               	mov	rax, rdx
;;   23:	 5d                   	pop	rbp
;;   24:	 c3                   	ret	
