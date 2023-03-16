;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 1)
	(i64.const 0)
	(i64.div_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c100000000       	mov	rcx, 0
;;    b:	 48c7c001000000       	mov	rax, 1
;;   12:	 4831d2               	xor	rdx, rdx
;;   15:	 48f7f1               	div	rcx
;;   18:	 5d                   	pop	rbp
;;   19:	 c3                   	ret	
