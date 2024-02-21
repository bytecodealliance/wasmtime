;;! target="x86_64"

(module
    (type $param-i32 (func (param i32)))

    (func $param-i32 (type $param-i32))
    (func (export "")
        (local i32)
        local.get 0
        (call_indirect (type $param-i32) (i32.const 0))
    )

    (table funcref
      (elem
        $param-i32)
    )
)

;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x36
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87cd000000         	ja	0xeb
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 48c7042400000000     	mov	qword ptr [rsp], 0
;;      	 448b5c2404           	mov	r11d, dword ptr [rsp + 4]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 b900000000           	mov	ecx, 0
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f8399000000         	jae	0xed
;;   54:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f852a000000         	jne	0xa1
;;   77:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 8b1424               	mov	edx, dword ptr [rsp]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742414           	mov	r14, qword ptr [rsp + 0x14]
;;      	 e904000000           	jmp	0xa5
;;   a1:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4885c0               	test	rax, rax
;;      	 0f8441000000         	je	0xef
;;   ae:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;      	 418b0b               	mov	ecx, dword ptr [r11]
;;      	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;      	 39d1                 	cmp	ecx, edx
;;      	 0f8531000000         	jne	0xf1
;;   c0:	 488b5820             	mov	rbx, qword ptr [rax + 0x20]
;;      	 488b4810             	mov	rcx, qword ptr [rax + 0x10]
;;      	 4883ec04             	sub	rsp, 4
;;      	 4889df               	mov	rdi, rbx
;;      	 4c89f6               	mov	rsi, r14
;;      	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;      	 ffd1                 	call	rcx
;;      	 4883c404             	add	rsp, 4
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742410           	mov	r14, qword ptr [rsp + 0x10]
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   eb:	 0f0b                 	ud2	
;;   ed:	 0f0b                 	ud2	
;;   ef:	 0f0b                 	ud2	
;;   f1:	 0f0b                 	ud2	
