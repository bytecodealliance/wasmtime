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
;;   2d:	 e92e010000           	jmp	0x160
;;   32:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   36:	 83e802               	sub	eax, 2
;;   39:	 50                   	push	rax
;;   3a:	 b900000000           	mov	ecx, 0
;;   3f:	 4c89f2               	mov	rdx, r14
;;   42:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   45:	 39d9                 	cmp	ecx, ebx
;;   47:	 0f8319010000         	jae	0x166
;;   4d:	 4189cb               	mov	r11d, ecx
;;   50:	 4d6bdb08             	imul	r11, r11, 8
;;   54:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   58:	 4889d6               	mov	rsi, rdx
;;   5b:	 4c01da               	add	rdx, r11
;;   5e:	 39d9                 	cmp	ecx, ebx
;;   60:	 480f43d6             	cmovae	rdx, rsi
;;   64:	 488b02               	mov	rax, qword ptr [rdx]
;;   67:	 4885c0               	test	rax, rax
;;   6a:	 0f8528000000         	jne	0x98
;;   70:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   74:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   78:	 4156                 	push	r14
;;   7a:	 51                   	push	rcx
;;   7b:	 4883ec08             	sub	rsp, 8
;;   7f:	 488b7c2410           	mov	rdi, qword ptr [rsp + 0x10]
;;   84:	 be00000000           	mov	esi, 0
;;   89:	 8b542408             	mov	edx, dword ptr [rsp + 8]
;;   8d:	 ffd3                 	call	rbx
;;   8f:	 4883c418             	add	rsp, 0x18
;;   93:	 e904000000           	jmp	0x9c
;;   98:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   9c:	 4885c0               	test	rax, rax
;;   9f:	 0f84c3000000         	je	0x168
;;   a5:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;   a9:	 418b0b               	mov	ecx, dword ptr [r11]
;;   ac:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;   af:	 39d1                 	cmp	ecx, edx
;;   b1:	 0f85b3000000         	jne	0x16a
;;   b7:	 50                   	push	rax
;;   b8:	 59                   	pop	rcx
;;   b9:	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;   bd:	 4883ec08             	sub	rsp, 8
;;   c1:	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;   c5:	 ffd2                 	call	rdx
;;   c7:	 4883c410             	add	rsp, 0x10
;;   cb:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   cf:	 83e901               	sub	ecx, 1
;;   d2:	 50                   	push	rax
;;   d3:	 51                   	push	rcx
;;   d4:	 b900000000           	mov	ecx, 0
;;   d9:	 4c89f2               	mov	rdx, r14
;;   dc:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   df:	 39d9                 	cmp	ecx, ebx
;;   e1:	 0f8385000000         	jae	0x16c
;;   e7:	 4189cb               	mov	r11d, ecx
;;   ea:	 4d6bdb08             	imul	r11, r11, 8
;;   ee:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   f2:	 4889d6               	mov	rsi, rdx
;;   f5:	 4c01da               	add	rdx, r11
;;   f8:	 39d9                 	cmp	ecx, ebx
;;   fa:	 480f43d6             	cmovae	rdx, rsi
;;   fe:	 488b02               	mov	rax, qword ptr [rdx]
;;  101:	 4885c0               	test	rax, rax
;;  104:	 0f8523000000         	jne	0x12d
;;  10a:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;  10e:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;  112:	 4156                 	push	r14
;;  114:	 51                   	push	rcx
;;  115:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;  11a:	 be00000000           	mov	esi, 0
;;  11f:	 8b1424               	mov	edx, dword ptr [rsp]
;;  122:	 ffd3                 	call	rbx
;;  124:	 4883c410             	add	rsp, 0x10
;;  128:	 e904000000           	jmp	0x131
;;  12d:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;  131:	 4885c0               	test	rax, rax
;;  134:	 0f8434000000         	je	0x16e
;;  13a:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;  13e:	 418b0b               	mov	ecx, dword ptr [r11]
;;  141:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;  144:	 39d1                 	cmp	ecx, edx
;;  146:	 0f8524000000         	jne	0x170
;;  14c:	 50                   	push	rax
;;  14d:	 59                   	pop	rcx
;;  14e:	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;  152:	 8b3c24               	mov	edi, dword ptr [rsp]
;;  155:	 ffd2                 	call	rdx
;;  157:	 4883c408             	add	rsp, 8
;;  15b:	 59                   	pop	rcx
;;  15c:	 01c1                 	add	ecx, eax
;;  15e:	 89c8                 	mov	eax, ecx
;;  160:	 4883c410             	add	rsp, 0x10
;;  164:	 5d                   	pop	rbp
;;  165:	 c3                   	ret	
;;  166:	 0f0b                 	ud2	
;;  168:	 0f0b                 	ud2	
;;  16a:	 0f0b                 	ud2	
;;  16c:	 0f0b                 	ud2	
;;  16e:	 0f0b                 	ud2	
;;  170:	 0f0b                 	ud2	
