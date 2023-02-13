;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48b80000000000000080 	
;; 				movabs	rax, 0x8000000000000000
;;    e:	 486bc0ff             	imul	rax, rax, -1
;;   12:	 5d                   	pop	rbp
;;   13:	 c3                   	ret	
