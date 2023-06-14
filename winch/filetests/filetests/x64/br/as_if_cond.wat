;;! target = "x86_64"
(module
  (func (export "as-if-cond") (result i32)
    (block (result i32)
      (if (result i32) (br 0 (i32.const 2))
        (then (i32.const 0))
        (else (i32.const 1))
      )
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c002000000       	mov	rax, 2
;;   13:	 4883c408             	add	rsp, 8
;;   17:	 5d                   	pop	rbp
;;   18:	 c3                   	ret	
