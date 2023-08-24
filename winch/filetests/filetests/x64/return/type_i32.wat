;;! target = "x86_64"
(module
  (func $dummy)

  (func (export "type-i32-value") (result i32)
   (block (result i32) (i32.ctz (return (i32.const 1))))
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
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 4883c408             	add	rsp, 8
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
