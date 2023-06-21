;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-test-operand") (result i32)
    (i32.eqz (loop (result i32) (call $dummy) (i32.const 13)))
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
;;   19:	 b80d000000           	mov	eax, 0xd
;;   1e:	 83f800               	cmp	eax, 0
;;   21:	 b800000000           	mov	eax, 0
;;   26:	 400f94c0             	sete	al
;;   2a:	 4883c408             	add	rsp, 8
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	
