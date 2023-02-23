;;! target = "x86_64"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.sub)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;    d:	 48893424             	mov	qword ptr [rsp], rsi
;;   11:	 488b0424             	mov	rax, qword ptr [rsp]
;;   15:	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;   1a:	 4829c1               	sub	rcx, rax
;;   1d:	 4889c8               	mov	rax, rcx
;;   20:	 4883c410             	add	rsp, 0x10
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
