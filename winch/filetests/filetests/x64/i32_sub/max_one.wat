;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 0x80000000)
	(i32.const 1)
	(i32.sub)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 b800000080           	mov	eax, 0x80000000
;;    9:	 83e801               	sub	eax, 1
;;    c:	 5d                   	pop	rbp
;;    d:	 c3                   	ret	
