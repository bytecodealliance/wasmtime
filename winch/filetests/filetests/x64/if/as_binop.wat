;;! target = "x86_64"

(module
  (func $dummy)
  (func (export "as-binary-operand") (param i32 i32) (result i32)
    (i32.mul
      (if (result i32) (local.get 0)
        (then (call $dummy) (i32.const 3))
        (else (call $dummy) (i32.const -3))
      )
      (if (result i32) (local.get 1)
        (then (call $dummy) (i32.const 4))
        (else (call $dummy) (i32.const -5))
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
;;      	 0f877f000000         	ja	0x97
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 89742408             	mov	dword ptr [rsp + 8], esi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0x3f
;;   30:	 e800000000           	call	0x35
;;      	 b803000000           	mov	eax, 3
;;      	 e90a000000           	jmp	0x49
;;   3f:	 e800000000           	call	0x44
;;      	 b8fdffffff           	mov	eax, 0xfffffffd
;;      	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8417000000         	je	0x73
;;   5c:	 4883ec0c             	sub	rsp, 0xc
;;      	 e800000000           	call	0x65
;;      	 4883c40c             	add	rsp, 0xc
;;      	 b804000000           	mov	eax, 4
;;      	 e912000000           	jmp	0x85
;;   73:	 4883ec0c             	sub	rsp, 0xc
;;      	 e800000000           	call	0x7c
;;      	 4883c40c             	add	rsp, 0xc
;;      	 b8fbffffff           	mov	eax, 0xfffffffb
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 0fafc8               	imul	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   97:	 0f0b                 	ud2	
