;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 7)
	(i32.const 5)
	(i32.rem_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b905000000           	mov	ecx, 5
;;      	 b807000000           	mov	eax, 7
;;      	 31d2                 	xor	edx, edx
;;      	 f7f1                 	div	ecx
;;      	 89d0                 	mov	eax, edx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
