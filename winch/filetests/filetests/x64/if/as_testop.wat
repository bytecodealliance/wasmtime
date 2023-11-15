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
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 85c0                 	test	eax, eax
;;   16:	 0f840f000000         	je	0x2b
;;   1c:	 e800000000           	call	0x21
;;   21:	 b80d000000           	mov	eax, 0xd
;;   26:	 e90a000000           	jmp	0x35
;;   2b:	 e800000000           	call	0x30
;;   30:	 b800000000           	mov	eax, 0
;;   35:	 83f800               	cmp	eax, 0
;;   38:	 b800000000           	mov	eax, 0
;;   3d:	 400f94c0             	sete	al
;;   41:	 4883c410             	add	rsp, 0x10
;;   45:	 5d                   	pop	rbp
;;   46:	 c3                   	ret	
