;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 1)
        (i64.eqz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 4883f800             	cmp	rax, 0
;;   17:	 b800000000           	mov	eax, 0
;;   1c:	 400f94c0             	sete	al
;;   20:	 4883c408             	add	rsp, 8
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
