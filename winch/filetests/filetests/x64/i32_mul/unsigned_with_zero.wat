;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const 1)
     	(i32.const 0)
    	(i32.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 b801000000           	mov	eax, 1
;;    9:	 6bc000               	imul	eax, eax, 0
;;    c:	 5d                   	pop	rbp
;;    d:	 c3                   	ret	
