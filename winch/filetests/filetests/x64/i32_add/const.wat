;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 10)
	(i32.const 20)
	(i32.add)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b80a000000           	mov	eax, 0xa
;;      	 83c014               	add	eax, 0x14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
