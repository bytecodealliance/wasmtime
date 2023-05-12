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
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;    d:	 48893424             	mov	qword ptr [rsp], rsi
;;   11:	 488b0c24             	mov	rcx, qword ptr [rsp]
;;   15:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   1a:	 4899                 	cqo	
;;   1c:	 4883f9ff             	cmp	rcx, -1
;;   20:	 0f850a000000         	jne	0x30
;;   26:	 ba00000000           	mov	edx, 0
;;   2b:	 e903000000           	jmp	0x33
;;   30:	 48f7f9               	idiv	rcx
;;   33:	 4889d0               	mov	rax, rdx
;;   36:	 4883c410             	add	rsp, 0x10
;;   3a:	 5d                   	pop	rbp
;;   3b:	 c3                   	ret	
