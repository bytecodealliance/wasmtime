;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 7)
	(i64.const 5)
	(i64.rem_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c105000000       	mov	rcx, 5
;;    b:	 48c7c007000000       	mov	rax, 7
;;   12:	 4831d2               	xor	rdx, rdx
;;   15:	 48f7f1               	div	rcx
;;   18:	 4889d0               	mov	rax, rdx
;;   1b:	 5d                   	pop	rbp
;;   1c:	 c3                   	ret	
