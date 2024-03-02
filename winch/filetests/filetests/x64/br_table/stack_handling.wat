;;! target = "x86_64"
(module
  (func (;0;) (param i32)
    local.get 0
    block ;; label = @1
      i32.const 808727609
      br_table 0 (;@1;) 1 (;@0;) 0 (;@1;)
    end
    drop
  )
  (export "main" (func 0))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c34c000000       	add	r11, 0x4c
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f879e000000         	ja	0xb9
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2440           	mov	qword ptr [rsp + 0x40], rdi
;;      	 4889742438           	mov	qword ptr [rsp + 0x38], rsi
;;      	 89542434             	mov	dword ptr [rsp + 0x34], edx
;;      	 448b5c2434           	mov	r11d, dword ptr [rsp + 0x34]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 b839343430           	mov	eax, 0x30343439
;;      	 b902000000           	mov	ecx, 2
;;      	 39c1                 	cmp	ecx, eax
;;      	 0f42c1               	cmovb	eax, ecx
;;      	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;      	 49630c83             	movsxd	rcx, dword ptr [r11 + rax*4]
;;      	 4901cb               	add	r11, rcx
;;      	 41ffe3               	jmp	r11
;;   79:	 1a00                 	sbb	al, byte ptr [rax]
;;      	 0000                 	add	byte ptr [rax], al
;;      	 1100                 	adc	dword ptr [rax], eax
;;      	 0000                 	add	byte ptr [rax], al
;;      	 1a00                 	sbb	al, byte ptr [rax]
;;      	 0000                 	add	byte ptr [rax], al
;;      	 e909000000           	jmp	0x93
;;   8a:	 4883c404             	add	rsp, 4
;;      	 e904000000           	jmp	0x97
;;   93:	 4883c404             	add	rsp, 4
;;      	 4883c418             	add	rsp, 0x18
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   b9:	 0f0b                 	ud2	
