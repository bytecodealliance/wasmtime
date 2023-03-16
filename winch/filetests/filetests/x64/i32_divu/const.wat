;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 20)
	(i32.const 10)
	(i32.div_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 b90a000000           	mov	ecx, 0xa
;;    9:	 b814000000           	mov	eax, 0x14
;;    e:	 31d2                 	xor	edx, edx
;;   10:	 f7f1                 	div	ecx
;;   12:	 5d                   	pop	rbp
;;   13:	 c3                   	ret	
