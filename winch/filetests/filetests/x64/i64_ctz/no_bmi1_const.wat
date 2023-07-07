;;! target = "x86_64"

(module
    (func (result i64)
        (i64.const 1)
        (i64.ctz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 480fbcc0             	bsf	rax, rax
;;   17:	 41bb00000000         	mov	r11d, 0
;;   1d:	 410f94c3             	sete	r11b
;;   21:	 49c1e306             	shl	r11, 6
;;   25:	 4c01d8               	add	rax, r11
;;   28:	 4883c408             	add	rsp, 8
;;   2c:	 5d                   	pop	rbp
;;   2d:	 c3                   	ret	
