;;! target = "x86_64"

(module
    (func (param i64) (result i32)
        (local.get 0)
        (i64.eqz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;    d:	 4c893424             	mov	qword ptr [rsp], r14
;;   11:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   16:	 4883f800             	cmp	rax, 0
;;   1a:	 b800000000           	mov	eax, 0
;;   1f:	 400f94c0             	sete	al
;;   23:	 4883c410             	add	rsp, 0x10
;;   27:	 5d                   	pop	rbp
;;   28:	 c3                   	ret	
