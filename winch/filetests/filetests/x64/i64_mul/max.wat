;;! target = "x86_64"
(module
    (func (result i64)
	(i64.const 0x7fffffffffffffff)
	(i64.const -1)
	(i64.mul)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48b8ffffffffffffff7f 	
;; 				movabs	rax, 0x7fffffffffffffff
;;      	 486bc0ff             	imul	rax, rax, -1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
