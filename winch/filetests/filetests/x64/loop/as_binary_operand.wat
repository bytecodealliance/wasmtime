;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-binary-operand") (result i32)
    (i32.mul
      (loop (result i32) (call $dummy) (i32.const 3))
      (loop (result i32) (call $dummy) (i32.const 4))
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
;;   26:	 b803000000           	mov	eax, 3
;;   2b:	 6bc004               	imul	eax, eax, 4
;;   2e:	 4883c408             	add	rsp, 8
;;   32:	 5d                   	pop	rbp
;;   33:	 c3                   	ret	
