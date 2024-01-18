;;! target="x86_64"

(module
  (type $over-i32 (func (param i32) (result i32)))

  (table funcref
    (elem
      $fib-i32
    )
  )
  
  (func $fib-i32 (export "fib-i32") (type $over-i32)
    (if (result i32) (i32.le_u (local.get 0) (i32.const 1))
      (then (i32.const 1))
      (else
        (i32.add
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 2))
            (i32.const 0)
          )
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 1))
            (i32.const 0)
          )
        )
      )
    )
  )
)


;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8793010000         	ja	0x1ab
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 83f801               	cmp	eax, 1
;;      	 b800000000           	mov	eax, 0
;;      	 400f96c0             	setbe	al
;;      	 85c0                 	test	eax, eax
;;      	 0f840a000000         	je	0x42
;;   38:	 b801000000           	mov	eax, 1
;;      	 e963010000           	jmp	0x1a5
;;   42:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 83e802               	sub	eax, 2
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 b900000000           	mov	ecx, 0
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f834a010000         	jae	0x1ad
;;   63:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8529000000         	jne	0xaf
;;   86:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4156                 	push	r14
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 488b7c2404           	mov	rdi, qword ptr [rsp + 4]
;;      	 be00000000           	mov	esi, 0
;;      	 8b1424               	mov	edx, dword ptr [rsp]
;;      	 ffd3                 	call	rbx
;;      	 4883c40c             	add	rsp, 0xc
;;      	 e904000000           	jmp	0xb3
;;   af:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4885c0               	test	rax, rax
;;      	 0f84f3000000         	je	0x1af
;;   bc:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;      	 418b0b               	mov	ecx, dword ptr [r11]
;;      	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;      	 39d1                 	cmp	ecx, edx
;;      	 0f85e3000000         	jne	0x1b1
;;   ce:	 50                   	push	rax
;;      	 59                   	pop	rcx
;;      	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 8b7c240c             	mov	edi, dword ptr [rsp + 0xc]
;;      	 ffd2                 	call	rdx
;;      	 4883c40c             	add	rsp, 0xc
;;      	 4883c404             	add	rsp, 4
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 83e901               	sub	ecx, 1
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 b900000000           	mov	ecx, 0
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f83a5000000         	jae	0x1b3
;;  10e:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8532000000         	jne	0x163
;;  131:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4156                 	push	r14
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 488b7c2410           	mov	rdi, qword ptr [rsp + 0x10]
;;      	 be00000000           	mov	esi, 0
;;      	 8b54240c             	mov	edx, dword ptr [rsp + 0xc]
;;      	 ffd3                 	call	rbx
;;      	 4883c40c             	add	rsp, 0xc
;;      	 4883c40c             	add	rsp, 0xc
;;      	 e904000000           	jmp	0x167
;;  163:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4885c0               	test	rax, rax
;;      	 0f8445000000         	je	0x1b5
;;  170:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;      	 418b0b               	mov	ecx, dword ptr [r11]
;;      	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;      	 39d1                 	cmp	ecx, edx
;;      	 0f8535000000         	jne	0x1b7
;;  182:	 50                   	push	rax
;;      	 59                   	pop	rcx
;;      	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;      	 4883ec08             	sub	rsp, 8
;;      	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;      	 ffd2                 	call	rdx
;;      	 4883c408             	add	rsp, 8
;;      	 4883c404             	add	rsp, 4
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 01c1                 	add	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  1ab:	 0f0b                 	ud2	
;;  1ad:	 0f0b                 	ud2	
;;  1af:	 0f0b                 	ud2	
;;  1b1:	 0f0b                 	ud2	
;;  1b3:	 0f0b                 	ud2	
;;  1b5:	 0f0b                 	ud2	
;;  1b7:	 0f0b                 	ud2	
