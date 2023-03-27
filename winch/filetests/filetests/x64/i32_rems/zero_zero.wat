;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 0)
	(i32.const 0)
	(i32.rem_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 b900000000           	mov	ecx, 0
;;    9:	 b800000000           	mov	eax, 0
;;    e:	 99                   	cdq	
;;    f:	 83f9ff               	cmp	ecx, -1
;;   12:	 0f850a000000         	jne	0x22
;;   18:	 ba00000000           	mov	edx, 0
;;   1d:	 e902000000           	jmp	0x24
;;   22:	 f7f9                 	idiv	ecx
;;   24:	 4889d0               	mov	rax, rdx
;;   27:	 5d                   	pop	rbp
;;   28:	 c3                   	ret	
