;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-test-operand") (param i32) (result i32)
    (i32.eqz
      (if (result i32) (local.get 0)
        (then (call $dummy) (i32.const 13))
        (else (call $dummy) (i32.const 0))
      )
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
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 85c0                 	test	eax, eax
;;   17:	 0f840f000000         	je	0x2c
;;   1d:	 e800000000           	call	0x22
;;   22:	 b80d000000           	mov	eax, 0xd
;;   27:	 e90a000000           	jmp	0x36
;;   2c:	 e800000000           	call	0x31
;;   31:	 b800000000           	mov	eax, 0
;;   36:	 83f800               	cmp	eax, 0
;;   39:	 b800000000           	mov	eax, 0
;;   3e:	 400f94c0             	sete	al
;;   42:	 4883c410             	add	rsp, 0x10
;;   46:	 5d                   	pop	rbp
;;   47:	 c3                   	ret	
