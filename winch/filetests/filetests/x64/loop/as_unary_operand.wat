;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-unary-operand") (result i32)
    (i32.ctz (loop (result i32) (call $dummy) (i32.const 13)))
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
;;   1e:	 0fbcc0               	bsf	eax, eax
;;   21:	 41bb00000000         	mov	r11d, 0
;;   27:	 410f94c3             	sete	r11b
;;   2b:	 41c1e305             	shl	r11d, 5
;;   2f:	 4401d8               	add	eax, r11d
;;   32:	 4883c408             	add	rsp, 8
;;   36:	 5d                   	pop	rbp
;;   37:	 c3                   	ret	
