;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 0x80000000)
	(i32.const -1)
	(i32.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b9ffffffff           	mov	ecx, 0xffffffff
;;   11:	 b800000080           	mov	eax, 0x80000000
;;   16:	 83f900               	cmp	ecx, 0
;;   19:	 0f8409000000         	je	0x28
;;   1f:	 99                   	cdq	
;;   20:	 f7f9                 	idiv	ecx
;;   22:	 4883c408             	add	rsp, 8
;;   26:	 5d                   	pop	rbp
;;   27:	 c3                   	ret	
;;   28:	 0f0b                 	ud2	
