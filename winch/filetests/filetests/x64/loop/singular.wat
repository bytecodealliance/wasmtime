;;! target = "x86_64"

(module
  (func (export "singular") (result i32)
    (loop (nop))
    (loop (result i32) (i32.const 7))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b807000000           	mov	eax, 7
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
