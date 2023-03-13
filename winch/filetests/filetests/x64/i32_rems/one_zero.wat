;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 1)
	(i32.const 0)
	(i32.rem_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 b900000000           	mov	ecx, 0
;;    9:	 b801000000           	mov	eax, 1
;;    e:	 83f900               	cmp	ecx, 0
;;   11:	 0f8502000000         	jne	0x19
;;   17:	 0f0b                 	ud2	
;;   19:	 99                   	cdq	
;;   1a:	 83f9ff               	cmp	ecx, -1
;;   1d:	 0f850a000000         	jne	0x2d
;;   23:	 ba00000000           	mov	edx, 0
;;   28:	 e902000000           	jmp	0x2f
;;   2d:	 f7f9                 	idiv	ecx
;;   2f:	 4889d0               	mov	rax, rdx
;;   32:	 5d                   	pop	rbp
;;   33:	 c3                   	ret	
