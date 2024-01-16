;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const -1)
	(i32.const -1)
	(i32.add)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b8ffffffff           	mov	eax, 0xffffffff
;;      	 83c0ff               	add	eax, -1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
