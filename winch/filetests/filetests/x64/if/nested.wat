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
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870a000000         	ja	0x22
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   22:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87bb000000         	ja	0xd3
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 89742408             	mov	dword ptr [rsp + 8], esi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 85c0                 	test	eax, eax
;;      	 0f8451000000         	je	0x81
;;   30:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x41
;;   3c:	 e800000000           	call	0x41
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x52
;;      	 e905000000           	jmp	0x57
;;   52:	 e800000000           	call	0x57
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0x72
;;   63:	 e800000000           	call	0x68
;;      	 b809000000           	mov	eax, 9
;;      	 e95b000000           	jmp	0xcd
;;   72:	 e800000000           	call	0x77
;;      	 b80a000000           	mov	eax, 0xa
;;      	 e94c000000           	jmp	0xcd
;;   81:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0x92
;;   8d:	 e800000000           	call	0x92
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f8405000000         	je	0xa3
;;      	 e905000000           	jmp	0xa8
;;   a3:	 e800000000           	call	0xa8
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0xc3
;;   b4:	 e800000000           	call	0xb9
;;      	 b80a000000           	mov	eax, 0xa
;;      	 e90a000000           	jmp	0xcd
;;   c3:	 e800000000           	call	0xc8
;;      	 b80b000000           	mov	eax, 0xb
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   d3:	 0f0b                 	ud2	
