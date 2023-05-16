;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const -1)
	(i64.const -1)
	(i64.div_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;   13:	 48c7c0ffffffff       	mov	rax, 0xffffffffffffffff
;;   1a:	 4831d2               	xor	rdx, rdx
;;   1d:	 48f7f1               	div	rcx
;;   20:	 4883c408             	add	rsp, 8
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
