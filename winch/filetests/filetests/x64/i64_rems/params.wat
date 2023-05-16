;;! target = "x86_64"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.rem_s)
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
;;   20:	 4899                 	cqo	
;;   22:	 4883f9ff             	cmp	rcx, -1
;;   26:	 0f850a000000         	jne	0x36
;;   2c:	 ba00000000           	mov	edx, 0
;;   31:	 e903000000           	jmp	0x39
;;   36:	 48f7f9               	idiv	rcx
;;   39:	 4889d0               	mov	rax, rdx
;;   3c:	 4883c418             	add	rsp, 0x18
;;   40:	 5d                   	pop	rbp
;;   41:	 c3                   	ret	
