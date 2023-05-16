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
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b905000000           	mov	ecx, 5
;;   11:	 b807000000           	mov	eax, 7
;;   16:	 31d2                 	xor	edx, edx
;;   18:	 f7f1                 	div	ecx
;;   1a:	 4889d0               	mov	rax, rdx
;;   1d:	 4883c408             	add	rsp, 8
;;   21:	 5d                   	pop	rbp
;;   22:	 c3                   	ret	
