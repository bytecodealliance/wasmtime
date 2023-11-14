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


;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 83f801               	cmp	eax, 1
;;   17:	 b800000000           	mov	eax, 0
;;   1c:	 400f96c0             	setbe	al
;;   20:	 85c0                 	test	eax, eax
;;   22:	 0f840a000000         	je	0x32
;;   28:	 b801000000           	mov	eax, 1
;;   2d:	 e963010000           	jmp	0x195
;;   32:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   36:	 83e802               	sub	eax, 2
;;   39:	 4883ec04             	sub	rsp, 4
;;   3d:	 890424               	mov	dword ptr [rsp], eax
;;   40:	 b900000000           	mov	ecx, 0
;;   45:	 4c89f2               	mov	rdx, r14
;;   48:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   4b:	 39d9                 	cmp	ecx, ebx
;;   4d:	 0f8348010000         	jae	0x19b
;;   53:	 4189cb               	mov	r11d, ecx
;;   56:	 4d6bdb08             	imul	r11, r11, 8
;;   5a:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   5e:	 4889d6               	mov	rsi, rdx
;;   61:	 4c01da               	add	rdx, r11
;;   64:	 39d9                 	cmp	ecx, ebx
;;   66:	 480f43d6             	cmovae	rdx, rsi
;;   6a:	 488b02               	mov	rax, qword ptr [rdx]
;;   6d:	 4885c0               	test	rax, rax
;;   70:	 0f8529000000         	jne	0x9f
;;   76:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   7a:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   7e:	 4156                 	push	r14
;;   80:	 4883ec04             	sub	rsp, 4
;;   84:	 890c24               	mov	dword ptr [rsp], ecx
;;   87:	 488b7c2404           	mov	rdi, qword ptr [rsp + 4]
;;   8c:	 be00000000           	mov	esi, 0
;;   91:	 8b1424               	mov	edx, dword ptr [rsp]
;;   94:	 ffd3                 	call	rbx
;;   96:	 4883c40c             	add	rsp, 0xc
;;   9a:	 e904000000           	jmp	0xa3
;;   9f:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   a3:	 4885c0               	test	rax, rax
;;   a6:	 0f84f1000000         	je	0x19d
;;   ac:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;   b0:	 418b0b               	mov	ecx, dword ptr [r11]
;;   b3:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;   b6:	 39d1                 	cmp	ecx, edx
;;   b8:	 0f85e1000000         	jne	0x19f
;;   be:	 50                   	push	rax
;;   bf:	 59                   	pop	rcx
;;   c0:	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;   c4:	 4883ec0c             	sub	rsp, 0xc
;;   c8:	 8b7c240c             	mov	edi, dword ptr [rsp + 0xc]
;;   cc:	 ffd2                 	call	rdx
;;   ce:	 4883c40c             	add	rsp, 0xc
;;   d2:	 4883c404             	add	rsp, 4
;;   d6:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   da:	 83e901               	sub	ecx, 1
;;   dd:	 4883ec04             	sub	rsp, 4
;;   e1:	 890424               	mov	dword ptr [rsp], eax
;;   e4:	 4883ec04             	sub	rsp, 4
;;   e8:	 890c24               	mov	dword ptr [rsp], ecx
;;   eb:	 b900000000           	mov	ecx, 0
;;   f0:	 4c89f2               	mov	rdx, r14
;;   f3:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   f6:	 39d9                 	cmp	ecx, ebx
;;   f8:	 0f83a3000000         	jae	0x1a1
;;   fe:	 4189cb               	mov	r11d, ecx
;;  101:	 4d6bdb08             	imul	r11, r11, 8
;;  105:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;  109:	 4889d6               	mov	rsi, rdx
;;  10c:	 4c01da               	add	rdx, r11
;;  10f:	 39d9                 	cmp	ecx, ebx
;;  111:	 480f43d6             	cmovae	rdx, rsi
;;  115:	 488b02               	mov	rax, qword ptr [rdx]
;;  118:	 4885c0               	test	rax, rax
;;  11b:	 0f8532000000         	jne	0x153
;;  121:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;  125:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;  129:	 4156                 	push	r14
;;  12b:	 4883ec04             	sub	rsp, 4
;;  12f:	 890c24               	mov	dword ptr [rsp], ecx
;;  132:	 4883ec0c             	sub	rsp, 0xc
;;  136:	 488b7c2410           	mov	rdi, qword ptr [rsp + 0x10]
;;  13b:	 be00000000           	mov	esi, 0
;;  140:	 8b54240c             	mov	edx, dword ptr [rsp + 0xc]
;;  144:	 ffd3                 	call	rbx
;;  146:	 4883c40c             	add	rsp, 0xc
;;  14a:	 4883c40c             	add	rsp, 0xc
;;  14e:	 e904000000           	jmp	0x157
;;  153:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;  157:	 4885c0               	test	rax, rax
;;  15a:	 0f8443000000         	je	0x1a3
;;  160:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;  164:	 418b0b               	mov	ecx, dword ptr [r11]
;;  167:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;  16a:	 39d1                 	cmp	ecx, edx
;;  16c:	 0f8533000000         	jne	0x1a5
;;  172:	 50                   	push	rax
;;  173:	 59                   	pop	rcx
;;  174:	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;  178:	 4883ec08             	sub	rsp, 8
;;  17c:	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;  180:	 ffd2                 	call	rdx
;;  182:	 4883c408             	add	rsp, 8
;;  186:	 4883c404             	add	rsp, 4
;;  18a:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;  18d:	 4883c404             	add	rsp, 4
;;  191:	 01c1                 	add	ecx, eax
;;  193:	 89c8                 	mov	eax, ecx
;;  195:	 4883c410             	add	rsp, 0x10
;;  199:	 5d                   	pop	rbp
;;  19a:	 c3                   	ret	
;;  19b:	 0f0b                 	ud2	
;;  19d:	 0f0b                 	ud2	
;;  19f:	 0f0b                 	ud2	
;;  1a1:	 0f0b                 	ud2	
;;  1a3:	 0f0b                 	ud2	
;;  1a5:	 0f0b                 	ud2	
