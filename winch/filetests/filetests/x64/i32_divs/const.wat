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
;;   11:	 0f8502000000         	jne	0x19
;;   17:	 0f0b                 	ud2	
;;   19:	 83f9ff               	cmp	ecx, -1
;;   1c:	 0f850e000000         	jne	0x30
;;   22:	 81f800000080         	cmp	eax, 0x80000000
;;   28:	 0f8502000000         	jne	0x30
;;   2e:	 0f0b                 	ud2	
;;   30:	 99                   	cdq	
;;   31:	 f7f9                 	idiv	ecx
;;   33:	 5d                   	pop	rbp
;;   34:	 c3                   	ret	
