;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 2)
        (i64.const 3)
        (i64.ne)
    )
)

;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c002000000       	mov	rax, 2
;;   13:	 4883f803             	cmp	rax, 3
;;   17:	 b800000000           	mov	eax, 0
;;   1c:	 400f95c0             	setne	al
;;   20:	 4883c408             	add	rsp, 8
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
