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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 4883c410             	add	rsp, 0x10
;;   14:	 5d                   	pop	rbp
;;   15:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b900000000           	mov	ecx, 0
;;   1a:	 4c89f2               	mov	rdx, r14
;;   1d:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   20:	 39d9                 	cmp	ecx, ebx
;;   22:	 0f8394000000         	jae	0xbc
;;   28:	 4189cb               	mov	r11d, ecx
;;   2b:	 4d6bdb08             	imul	r11, r11, 8
;;   2f:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   33:	 4889d6               	mov	rsi, rdx
;;   36:	 4c01da               	add	rdx, r11
;;   39:	 39d9                 	cmp	ecx, ebx
;;   3b:	 480f43d6             	cmovae	rdx, rsi
;;   3f:	 488b02               	mov	rax, qword ptr [rdx]
;;   42:	 4885c0               	test	rax, rax
;;   45:	 0f8536000000         	jne	0x81
;;   4b:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   4f:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   53:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   58:	 4883ec04             	sub	rsp, 4
;;   5c:	 44891c24             	mov	dword ptr [rsp], r11d
;;   60:	 4156                 	push	r14
;;   62:	 4883ec04             	sub	rsp, 4
;;   66:	 890c24               	mov	dword ptr [rsp], ecx
;;   69:	 488b7c2404           	mov	rdi, qword ptr [rsp + 4]
;;   6e:	 be00000000           	mov	esi, 0
;;   73:	 8b1424               	mov	edx, dword ptr [rsp]
;;   76:	 ffd3                 	call	rbx
;;   78:	 4883c40c             	add	rsp, 0xc
;;   7c:	 e904000000           	jmp	0x85
;;   81:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   85:	 4885c0               	test	rax, rax
;;   88:	 0f8430000000         	je	0xbe
;;   8e:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;   92:	 418b0b               	mov	ecx, dword ptr [r11]
;;   95:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;   98:	 39d1                 	cmp	ecx, edx
;;   9a:	 0f8520000000         	jne	0xc0
;;   a0:	 488b4810             	mov	rcx, qword ptr [rax + 0x10]
;;   a4:	 4883ec0c             	sub	rsp, 0xc
;;   a8:	 8b7c240c             	mov	edi, dword ptr [rsp + 0xc]
;;   ac:	 ffd1                 	call	rcx
;;   ae:	 4883c40c             	add	rsp, 0xc
;;   b2:	 4883c404             	add	rsp, 4

;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870e000000         	ja	0x26
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   26:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87b4000000         	ja	0xcc
;;   18:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 b900000000           	mov	ecx, 0
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f8389000000         	jae	0xce
;;   45:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8529000000         	jne	0x91
;;   68:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4156                 	push	r14
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 488b7c2404           	mov	rdi, qword ptr [rsp + 4]
;;      	 be00000000           	mov	esi, 0
;;      	 8b1424               	mov	edx, dword ptr [rsp]
;;      	 ffd3                 	call	rbx
;;      	 4883c40c             	add	rsp, 0xc
;;      	 e904000000           	jmp	0x95
;;   91:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4885c0               	test	rax, rax
;;      	 0f8432000000         	je	0xd0
;;   9e:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;      	 418b0b               	mov	ecx, dword ptr [r11]
;;      	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;      	 39d1                 	cmp	ecx, edx
;;      	 0f8522000000         	jne	0xd2
;;   b0:	 488b4810             	mov	rcx, qword ptr [rax + 0x10]
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 8b7c240c             	mov	edi, dword ptr [rsp + 0xc]
;;      	 ffd1                 	call	rcx
;;      	 4883c40c             	add	rsp, 0xc
;;      	 4883c404             	add	rsp, 4
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   cc:	 0f0b                 	ud2	
;;   ce:	 0f0b                 	ud2	
;;   d0:	 0f0b                 	ud2	
;;   d2:	 0f0b                 	ud2	
