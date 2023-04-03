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
;;    4:	 b9ffffffff           	mov	ecx, 0xffffffff
;;    9:	 b800000080           	mov	eax, 0x80000000
;;    e:	 83f900               	cmp	ecx, 0
;;   11:	 0f8405000000         	je	0x1c
;;   17:	 99                   	cdq	
;;   18:	 f7f9                 	idiv	ecx
;;   1a:	 5d                   	pop	rbp
;;   1b:	 c3                   	ret	
;;   1c:	 0f0b                 	ud2	
