;;! target = "x86_64"

(module
    (func (result i64)
        (i64.const 1)
        (i64.clz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 480fbdc0             	bsr	rax, rax
;;   17:	 41bb00000000         	mov	r11d, 0
;;   1d:	 410f95c3             	setne	r11b
;;   21:	 48f7d8               	neg	rax
;;   24:	 4883c040             	add	rax, 0x40
;;   28:	 4c29d8               	sub	rax, r11
;;   2b:	 4883c408             	add	rsp, 8
;;   2f:	 5d                   	pop	rbp
;;   30:	 c3                   	ret	
