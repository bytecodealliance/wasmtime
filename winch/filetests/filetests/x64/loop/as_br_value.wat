;;! target = "x86_64"
(module
  (func (export "as-br-value") (result i32)
    (block (result i32) (br 0 (loop (result i32) (i32.const 1))))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 4883c408             	add	rsp, 8
;;   17:	 5d                   	pop	rbp
;;   18:	 c3                   	ret	
