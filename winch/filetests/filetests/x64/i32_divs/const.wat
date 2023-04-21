;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 20)
	(i32.const 10)
	(i32.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 b90a000000           	mov	ecx, 0xa
;;    9:	 b814000000           	mov	eax, 0x14
;;    e:	 83f900               	cmp	ecx, 0
;;   11:	 0f8405000000         	je	0x1c
;;   17:	 99                   	cdq	
;;   18:	 f7f9                 	idiv	ecx
;;   1a:	 5d                   	pop	rbp
;;   1b:	 c3                   	ret	
;;   1c:	 0f0b                 	ud2	
