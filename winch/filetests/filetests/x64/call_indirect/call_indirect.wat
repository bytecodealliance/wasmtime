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
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 83f801               	cmp	eax, 1
;;   18:	 b800000000           	mov	eax, 0
;;   1d:	 400f96c0             	setbe	al
;;   21:	 85c0                 	test	eax, eax
;;   23:	 0f840a000000         	je	0x33
;;   29:	 b801000000           	mov	eax, 1
;;   2e:	 e92e010000           	jmp	0x161
;;   33:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   37:	 83e802               	sub	eax, 2
;;   3a:	 50                   	push	rax
;;   3b:	 b900000000           	mov	ecx, 0
;;   40:	 4c89f2               	mov	rdx, r14
;;   43:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   46:	 39d9                 	cmp	ecx, ebx
;;   48:	 0f8319010000         	jae	0x167
;;   4e:	 4189cb               	mov	r11d, ecx
;;   51:	 4d6bdb08             	imul	r11, r11, 8
;;   55:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   59:	 4889d6               	mov	rsi, rdx
;;   5c:	 4c01da               	add	rdx, r11
;;   5f:	 39d9                 	cmp	ecx, ebx
;;   61:	 480f43d6             	cmovae	rdx, rsi
;;   65:	 488b02               	mov	rax, qword ptr [rdx]
;;   68:	 4885c0               	test	rax, rax
;;   6b:	 0f8528000000         	jne	0x99
;;   71:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   75:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   79:	 4156                 	push	r14
;;   7b:	 51                   	push	rcx
;;   7c:	 4883ec08             	sub	rsp, 8
;;   80:	 488b7c2410           	mov	rdi, qword ptr [rsp + 0x10]
;;   85:	 be00000000           	mov	esi, 0
;;   8a:	 8b542408             	mov	edx, dword ptr [rsp + 8]
;;   8e:	 ffd3                 	call	rbx
;;   90:	 4883c418             	add	rsp, 0x18
;;   94:	 e904000000           	jmp	0x9d
;;   99:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   9d:	 4885c0               	test	rax, rax
;;   a0:	 0f84c3000000         	je	0x169
;;   a6:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;   aa:	 418b0b               	mov	ecx, dword ptr [r11]
;;   ad:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;   b0:	 39d1                 	cmp	ecx, edx
;;   b2:	 0f85b3000000         	jne	0x16b
;;   b8:	 50                   	push	rax
;;   b9:	 59                   	pop	rcx
;;   ba:	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;   be:	 4883ec08             	sub	rsp, 8
;;   c2:	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;   c6:	 ffd2                 	call	rdx
;;   c8:	 4883c410             	add	rsp, 0x10
;;   cc:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   d0:	 83e901               	sub	ecx, 1
;;   d3:	 50                   	push	rax
;;   d4:	 51                   	push	rcx
;;   d5:	 b900000000           	mov	ecx, 0
;;   da:	 4c89f2               	mov	rdx, r14
;;   dd:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   e0:	 39d9                 	cmp	ecx, ebx
;;   e2:	 0f8385000000         	jae	0x16d
;;   e8:	 4189cb               	mov	r11d, ecx
;;   eb:	 4d6bdb08             	imul	r11, r11, 8
;;   ef:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   f3:	 4889d6               	mov	rsi, rdx
;;   f6:	 4c01da               	add	rdx, r11
;;   f9:	 39d9                 	cmp	ecx, ebx
;;   fb:	 480f43d6             	cmovae	rdx, rsi
;;   ff:	 488b02               	mov	rax, qword ptr [rdx]
;;  102:	 4885c0               	test	rax, rax
;;  105:	 0f8523000000         	jne	0x12e
;;  10b:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;  10f:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;  113:	 4156                 	push	r14
;;  115:	 51                   	push	rcx
;;  116:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;  11b:	 be00000000           	mov	esi, 0
;;  120:	 8b1424               	mov	edx, dword ptr [rsp]
;;  123:	 ffd3                 	call	rbx
;;  125:	 4883c410             	add	rsp, 0x10
;;  129:	 e904000000           	jmp	0x132
;;  12e:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;  132:	 4885c0               	test	rax, rax
;;  135:	 0f8434000000         	je	0x16f
;;  13b:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;  13f:	 418b0b               	mov	ecx, dword ptr [r11]
;;  142:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;  145:	 39d1                 	cmp	ecx, edx
;;  147:	 0f8524000000         	jne	0x171
;;  14d:	 50                   	push	rax
;;  14e:	 59                   	pop	rcx
;;  14f:	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;  153:	 8b3c24               	mov	edi, dword ptr [rsp]
;;  156:	 ffd2                 	call	rdx
;;  158:	 4883c408             	add	rsp, 8
;;  15c:	 59                   	pop	rcx
;;  15d:	 01c1                 	add	ecx, eax
;;  15f:	 89c8                 	mov	eax, ecx
;;  161:	 4883c410             	add	rsp, 0x10
;;  165:	 5d                   	pop	rbp
;;  166:	 c3                   	ret	
;;  167:	 0f0b                 	ud2	
;;  169:	 0f0b                 	ud2	
;;  16b:	 0f0b                 	ud2	
;;  16d:	 0f0b                 	ud2	
;;  16f:	 0f0b                 	ud2	
;;  171:	 0f0b                 	ud2	
