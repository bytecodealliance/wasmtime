;;! target = "x86_64"

(module
    (func (param i64) (result i64)
        (local.get 0)
        (i64.ctz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;    d:	 4c893424             	mov	qword ptr [rsp], r14
;;   11:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   16:	 480fbcc0             	bsf	rax, rax
;;   1a:	 41bb00000000         	mov	r11d, 0
;;   20:	 410f94c3             	sete	r11b
;;   24:	 49c1e306             	shl	r11, 6
;;   28:	 4c01d8               	add	rax, r11
;;   2b:	 4883c410             	add	rsp, 0x10
;;   2f:	 5d                   	pop	rbp
;;   30:	 c3                   	ret	
