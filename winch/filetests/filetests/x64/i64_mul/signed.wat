;;! target = "x86_64"

(module
    (func (result i64)
        (i64.const -1)
	(i64.const -1)
	(i64.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c0ffffffff       	mov	rax, 0xffffffffffffffff
;;    b:	 486bc0ff             	imul	rax, rax, -1
;;    f:	 5d                   	pop	rbp
;;   10:	 c3                   	ret	
