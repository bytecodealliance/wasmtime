;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 1)
        (i32.wrap_i64)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 89c0                 	mov	eax, eax
;;   15:	 4883c408             	add	rsp, 8
;;   19:	 5d                   	pop	rbp
;;   1a:	 c3                   	ret	
