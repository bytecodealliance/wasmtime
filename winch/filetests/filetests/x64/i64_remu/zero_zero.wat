;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0)
	(i64.const 0)
	(i64.rem_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c100000000       	mov	rcx, 0
;;   13:	 48c7c000000000       	mov	rax, 0
;;   1a:	 4831d2               	xor	rdx, rdx
;;   1d:	 48f7f1               	div	rcx
;;   20:	 4889d0               	mov	rax, rdx
;;   23:	 4883c408             	add	rsp, 8
;;   27:	 5d                   	pop	rbp
;;   28:	 c3                   	ret	
