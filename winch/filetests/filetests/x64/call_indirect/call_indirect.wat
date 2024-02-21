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
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c330000000       	add	r11, 0x30
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87b4010000         	ja	0x1d2
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 83f801               	cmp	eax, 1
;;      	 b800000000           	mov	eax, 0
;;      	 400f96c0             	setbe	al
;;      	 85c0                 	test	eax, eax
;;      	 0f840a000000         	je	0x52
;;   48:	 b801000000           	mov	eax, 1
;;      	 e97a010000           	jmp	0x1cc
;;   52:	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 83e802               	sub	eax, 2
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 b900000000           	mov	ecx, 0
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f8361010000         	jae	0x1d4
;;   73:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f852a000000         	jne	0xc0
;;   96:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 8b1424               	mov	edx, dword ptr [rsp]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742414           	mov	r14, qword ptr [rsp + 0x14]
;;      	 e904000000           	jmp	0xc4
;;   c0:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4885c0               	test	rax, rax
;;      	 0f8409010000         	je	0x1d6
;;   cd:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;      	 418b0b               	mov	ecx, dword ptr [r11]
;;      	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;      	 39d1                 	cmp	ecx, edx
;;      	 0f85f9000000         	jne	0x1d8
;;   df:	 50                   	push	rax
;;      	 59                   	pop	rcx
;;      	 4c8b4120             	mov	r8, qword ptr [rcx + 0x20]
;;      	 488b5910             	mov	rbx, qword ptr [rcx + 0x10]
;;      	 4883ec04             	sub	rsp, 4
;;      	 4c89c7               	mov	rdi, r8
;;      	 4c89f6               	mov	rsi, r14
;;      	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742410           	mov	r14, qword ptr [rsp + 0x10]
;;      	 8b4c2404             	mov	ecx, dword ptr [rsp + 4]
;;      	 83e901               	sub	ecx, 1
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 b900000000           	mov	ecx, 0
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f83ac000000         	jae	0x1da
;;  12e:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8533000000         	jne	0x184
;;  151:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 8b54240c             	mov	edx, dword ptr [rsp + 0xc]
;;      	 ffd3                 	call	rbx
;;      	 4883c40c             	add	rsp, 0xc
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 e904000000           	jmp	0x188
;;  184:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4885c0               	test	rax, rax
;;      	 0f844b000000         	je	0x1dc
;;  191:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;      	 418b0b               	mov	ecx, dword ptr [r11]
;;      	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;      	 39d1                 	cmp	ecx, edx
;;      	 0f853b000000         	jne	0x1de
;;  1a3:	 50                   	push	rax
;;      	 59                   	pop	rcx
;;      	 4c8b4120             	mov	r8, qword ptr [rcx + 0x20]
;;      	 488b5910             	mov	rbx, qword ptr [rcx + 0x10]
;;      	 4c89c7               	mov	rdi, r8
;;      	 4c89f6               	mov	rsi, r14
;;      	 8b1424               	mov	edx, dword ptr [rsp]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742414           	mov	r14, qword ptr [rsp + 0x14]
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 01c1                 	add	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  1d2:	 0f0b                 	ud2	
;;  1d4:	 0f0b                 	ud2	
;;  1d6:	 0f0b                 	ud2	
;;  1d8:	 0f0b                 	ud2	
;;  1da:	 0f0b                 	ud2	
;;  1dc:	 0f0b                 	ud2	
;;  1de:	 0f0b                 	ud2	
