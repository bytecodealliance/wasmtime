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
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870e000000         	ja	0x29
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   29:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87bf000000         	ja	0xda
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 89742408             	mov	dword ptr [rsp + 8], esi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 85c0                 	test	eax, eax
;;      	 0f8451000000         	je	0x88
;;   37:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x48
;;   43:	 e800000000           	call	0x48
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x59
;;      	 e905000000           	jmp	0x5e
;;   59:	 e800000000           	call	0x5e
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0x79
;;   6a:	 e800000000           	call	0x6f
;;      	 b809000000           	mov	eax, 9
;;      	 e95b000000           	jmp	0xd4
;;   79:	 e800000000           	call	0x7e
;;      	 b80a000000           	mov	eax, 0xa
;;      	 e94c000000           	jmp	0xd4
;;   88:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x99
;;   94:	 e800000000           	call	0x99
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0xaa
;;      	 e905000000           	jmp	0xaf
;;   aa:	 e800000000           	call	0xaf
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0xca
;;   bb:	 e800000000           	call	0xc0
;;      	 b80a000000           	mov	eax, 0xa
;;      	 e90a000000           	jmp	0xd4
;;   ca:	 e800000000           	call	0xcf
;;      	 b80b000000           	mov	eax, 0xb
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   da:	 0f0b                 	ud2	
