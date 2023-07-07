;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const -1)
	(i32.const 1)
	(i32.mul)
     )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b8ffffffff           	mov	eax, 0xffffffff
;;   11:	 6bc001               	imul	eax, eax, 1
;;   14:	 4883c408             	add	rsp, 8
;;   18:	 5d                   	pop	rbp
;;   19:	 c3                   	ret	
