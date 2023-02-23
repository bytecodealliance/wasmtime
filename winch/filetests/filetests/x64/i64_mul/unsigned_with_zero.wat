;;! target = "x86_64"

(module
    (func (result i64)
        (i64.const 1)
     	(i64.const 0)
    	(i64.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c001000000       	mov	rax, 1
;;    b:	 486bc000             	imul	rax, rax, 0
;;    f:	 5d                   	pop	rbp
;;   10:	 c3                   	ret	
