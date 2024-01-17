;;! target = "x86_64"
(module
  (table $t3 3 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)
  (func $f3 (export "get-funcref") (param $i i32) (result funcref)
    (table.get $t3 (local.get $i))
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
;;      	 0f8779000000         	ja	0x91
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f8361000000         	jae	0x93
;;   32:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8532000000         	jne	0x87
;;   55:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4156                 	push	r14
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;      	 be00000000           	mov	esi, 0
;;      	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4883c40c             	add	rsp, 0xc
;;      	 e904000000           	jmp	0x8b
;;   87:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   91:	 0f0b                 	ud2	
;;   93:	 0f0b                 	ud2	
