;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const 1)
        (i32.eqz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 83f800               	cmp	eax, 0
;;   14:	 b800000000           	mov	eax, 0
;;   19:	 400f94c0             	sete	al
;;   1d:	 4883c408             	add	rsp, 8
;;   21:	 5d                   	pop	rbp
;;   22:	 c3                   	ret	
