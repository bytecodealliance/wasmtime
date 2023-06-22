;;! target = "x86_64"

(module
  (func $dummy)
  (func (export "multi") (result i32)
    (loop (call $dummy) (call $dummy) (call $dummy) (call $dummy))
    (loop (result i32) (call $dummy) (call $dummy) (i32.const 8) (call $dummy))
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
;;   26:	 4883ec08             	sub	rsp, 8
;;   2a:	 e800000000           	call	0x2f
;;   2f:	 4883c408             	add	rsp, 8
;;   33:	 4883ec08             	sub	rsp, 8
;;   37:	 e800000000           	call	0x3c
;;   3c:	 4883c408             	add	rsp, 8
;;   40:	 4883ec08             	sub	rsp, 8
;;   44:	 e800000000           	call	0x49
;;   49:	 4883c408             	add	rsp, 8
;;   4d:	 4883ec08             	sub	rsp, 8
;;   51:	 e800000000           	call	0x56
;;   56:	 4883c408             	add	rsp, 8
;;   5a:	 4883ec08             	sub	rsp, 8
;;   5e:	 e800000000           	call	0x63
;;   63:	 4883c408             	add	rsp, 8
;;   67:	 48c7c008000000       	mov	rax, 8
;;   6e:	 4883c408             	add	rsp, 8
;;   72:	 5d                   	pop	rbp
;;   73:	 c3                   	ret	
