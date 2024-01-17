;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "nested") (param i32 i32) (result i32)
    (if (result i32) (local.get 0)
      (then
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 9))
          (else (call $dummy) (i32.const 10))
        )
      )
      (else
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 10))
          (else (call $dummy) (i32.const 11))
        )
      )
    )
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 89742408             	mov	dword ptr [rsp + 8], esi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 85c0                 	test	eax, eax
;;      	 0f8451000000         	je	0x71
;;   20:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x31
;;   2c:	 e800000000           	call	0x31
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x42
;;      	 e905000000           	jmp	0x47
;;   42:	 e800000000           	call	0x47
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0x62
;;   53:	 e800000000           	call	0x58
;;      	 b809000000           	mov	eax, 9
;;      	 e95b000000           	jmp	0xbd
;;   62:	 e800000000           	call	0x67
;;      	 b80a000000           	mov	eax, 0xa
;;      	 e94c000000           	jmp	0xbd
;;   71:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x82
;;   7d:	 e800000000           	call	0x82
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x93
;;      	 e905000000           	jmp	0x98
;;   93:	 e800000000           	call	0x98
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0xb3
;;   a4:	 e800000000           	call	0xa9
;;      	 b80a000000           	mov	eax, 0xa
;;      	 e90a000000           	jmp	0xbd
;;   b3:	 e800000000           	call	0xb8
;;      	 b80b000000           	mov	eax, 0xb
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
