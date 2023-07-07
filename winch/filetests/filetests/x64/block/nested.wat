;;! target = "x86_64"
(module
  (func $dummy)

  (func (export "nested") (result i32)
    (block (result i32)
      (block (call $dummy) (block) (nop))
      (block (result i32) (call $dummy) (i32.const 9))
    )
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
;;    c:	 4883ec08             	sub	rsp, 8
;;   10:	 e800000000           	call	0x15
;;   15:	 4883c408             	add	rsp, 8
;;   19:	 4883ec08             	sub	rsp, 8
;;   1d:	 e800000000           	call	0x22
;;   22:	 4883c408             	add	rsp, 8
;;   26:	 48c7c009000000       	mov	rax, 9
;;   2d:	 4883c408             	add	rsp, 8
;;   31:	 5d                   	pop	rbp
;;   32:	 c3                   	ret	
