;;! target = "x86_64"
(module
    (func (result i64)
	(i64.const 0x7fffffffffffffff)
	(i64.const -1)
	(i64.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48b8ffffffffffffff7f 	
;; 				movabs	rax, 0x7fffffffffffffff
;;    e:	 486bc0ff             	imul	rax, rax, -1
;;   12:	 5d                   	pop	rbp
;;   13:	 c3                   	ret	
