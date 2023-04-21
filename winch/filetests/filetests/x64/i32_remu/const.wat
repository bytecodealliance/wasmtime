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
;;    e:	 31d2                 	xor	edx, edx
;;   10:	 f7f1                 	div	ecx
;;   12:	 4889d0               	mov	rax, rdx
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
