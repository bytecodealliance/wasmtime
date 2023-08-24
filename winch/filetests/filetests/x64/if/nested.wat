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
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   18:	 85c0                 	test	eax, eax
;;   1a:	 0f8451000000         	je	0x71
;;   20:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   24:	 85c0                 	test	eax, eax
;;   26:	 0f8405000000         	je	0x31
;;   2c:	 e800000000           	call	0x31
;;   31:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   35:	 85c0                 	test	eax, eax
;;   37:	 0f8405000000         	je	0x42
;;   3d:	 e905000000           	jmp	0x47
;;   42:	 e800000000           	call	0x47
;;   47:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   4b:	 85c0                 	test	eax, eax
;;   4d:	 0f840f000000         	je	0x62
;;   53:	 e800000000           	call	0x58
;;   58:	 b809000000           	mov	eax, 9
;;   5d:	 e95b000000           	jmp	0xbd
;;   62:	 e800000000           	call	0x67
;;   67:	 b80a000000           	mov	eax, 0xa
;;   6c:	 e94c000000           	jmp	0xbd
;;   71:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   75:	 85c0                 	test	eax, eax
;;   77:	 0f8405000000         	je	0x82
;;   7d:	 e800000000           	call	0x82
;;   82:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   86:	 85c0                 	test	eax, eax
;;   88:	 0f8405000000         	je	0x93
;;   8e:	 e905000000           	jmp	0x98
;;   93:	 e800000000           	call	0x98
;;   98:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   9c:	 85c0                 	test	eax, eax
;;   9e:	 0f840f000000         	je	0xb3
;;   a4:	 e800000000           	call	0xa9
;;   a9:	 b80a000000           	mov	eax, 0xa
;;   ae:	 e90a000000           	jmp	0xbd
;;   b3:	 e800000000           	call	0xb8
;;   b8:	 b80b000000           	mov	eax, 0xb
;;   bd:	 4883c410             	add	rsp, 0x10
;;   c1:	 5d                   	pop	rbp
;;   c2:	 c3                   	ret	
