;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const 2)
        (i32.const 3)
        (i32.le_u)
    )
)

;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b802000000           	mov	eax, 2
;;   11:	 83f803               	cmp	eax, 3
;;   14:	 b800000000           	mov	eax, 0
;;   19:	 400f96c0             	setbe	al
;;   1d:	 4883c408             	add	rsp, 8
;;   21:	 5d                   	pop	rbp
;;   22:	 c3                   	ret	
