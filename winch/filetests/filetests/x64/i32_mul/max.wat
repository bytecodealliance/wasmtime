;;! target = "x86_64"
(module
    (func (result i32)
	(i32.const 0x7fffffff)
	(i32.const -1)
	(i32.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 b8ffffff7f           	mov	eax, 0x7fffffff
;;    9:	 6bc0ff               	imul	eax, eax, -1
;;    c:	 5d                   	pop	rbp
;;    d:	 c3                   	ret	
