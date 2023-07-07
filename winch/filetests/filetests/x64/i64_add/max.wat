;;! target = "x86_64"
(module
    (func (result i64)
	(i64.const 1)
	(i64.const 0x7fffffffffffffff)
	(i64.add)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 49bbffffffffffffff7f 	
;; 				movabs	r11, 0x7fffffffffffffff
;;   1d:	 4c01d8               	add	rax, r11
;;   20:	 4883c408             	add	rsp, 8
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
