;;! target = "x86_64"
;;! flags = ["has_lzcnt"]

(module
    (func (result i64)
        (i64.const 1)
        (i64.clz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c001000000       	mov	rax, 1
;;      	 f3480fbdc0           	lzcnt	rax, rax
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
