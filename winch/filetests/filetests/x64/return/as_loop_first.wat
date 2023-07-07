;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-loop-first") (result i32)
   (loop (result i32) (return (i32.const 3)) (i32.const 2))
  )
)
  
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c003000000       	mov	rax, 3
;;   13:	 4883c408             	add	rsp, 8
;;   17:	 5d                   	pop	rbp
;;   18:	 c3                   	ret	
