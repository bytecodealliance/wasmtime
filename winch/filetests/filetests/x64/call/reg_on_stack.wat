;;! target = "x86_64"
(module
  (func (export "") (param i32) (result i32)
    local.get 0
    i32.const 1
    call 0
    i32.const 1
    call 0
    br_if 0 (;@0;)
    unreachable
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c324000000       	add	r11, 0x24
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8786000000         	ja	0xa4
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 448b5c2404           	mov	r11d, dword ptr [rsp + 4]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 4883ec04             	sub	rsp, 4
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 ba01000000           	mov	edx, 1
;;      	 e800000000           	call	0x51
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742414           	mov	r14, qword ptr [rsp + 0x14]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 ba01000000           	mov	edx, 1
;;      	 e800000000           	call	0x71
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 8b0424               	mov	eax, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8409000000         	je	0x9c
;;   93:	 4883c404             	add	rsp, 4
;;      	 e902000000           	jmp	0x9e
;;   9c:	 0f0b                 	ud2	
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   a4:	 0f0b                 	ud2	
