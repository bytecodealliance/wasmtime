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
;;   17:	 0f8411000000         	je	0x2e
;;   1d:	 e800000000           	call	0x22
;;   22:	 48c7c00d000000       	mov	rax, 0xd
;;   29:	 e90c000000           	jmp	0x3a
;;   2e:	 e800000000           	call	0x33
;;   33:	 48c7c000000000       	mov	rax, 0
;;   3a:	 83f800               	cmp	eax, 0
;;   3d:	 b800000000           	mov	eax, 0
;;   42:	 400f94c0             	sete	al
;;   46:	 4883c410             	add	rsp, 0x10
;;   4a:	 5d                   	pop	rbp
;;   4b:	 c3                   	ret	
