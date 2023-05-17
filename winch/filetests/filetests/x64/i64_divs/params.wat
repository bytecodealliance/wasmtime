;;! target = "x86_64"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;    d:	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;   12:	 4c893424             	mov	qword ptr [rsp], r14
;;   16:	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;   1b:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;   20:	 4883f900             	cmp	rcx, 0
;;   24:	 0f840b000000         	je	0x35
;;   2a:	 4899                 	cqo	
;;   2c:	 48f7f9               	idiv	rcx
;;   2f:	 4883c418             	add	rsp, 0x18
;;   33:	 5d                   	pop	rbp
;;   34:	 c3                   	ret	
;;   35:	 0f0b                 	ud2	
