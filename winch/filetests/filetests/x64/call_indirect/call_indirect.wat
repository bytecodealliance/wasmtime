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
;;   2e:	 e913010000           	jmp	0x146
;;   33:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   37:	 83e802               	sub	eax, 2
;;   3a:	 50                   	push	rax
;;   3b:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   3f:	 498b4b48             	mov	rcx, qword ptr [r11 + 0x48]
;;   43:	 bb00000000           	mov	ebx, 0
;;   48:	 4d89f1               	mov	r9, r14
;;   4b:	 4d8b4150             	mov	r8, qword ptr [r9 + 0x50]
;;   4f:	 4439c3               	cmp	ebx, r8d
;;   52:	 0f83f4000000         	jae	0x14c
;;   58:	 4189db               	mov	r11d, ebx
;;   5b:	 4d6bdb08             	imul	r11, r11, 8
;;   5f:	 4d8b4948             	mov	r9, qword ptr [r9 + 0x48]
;;   63:	 4d89ca               	mov	r10, r9
;;   66:	 4d01d9               	add	r9, r11
;;   69:	 4439c3               	cmp	ebx, r8d
;;   6c:	 4d0f43ca             	cmovae	r9, r10
;;   70:	 4d8b09               	mov	r9, qword ptr [r9]
;;   73:	 4c89c8               	mov	rax, r9
;;   76:	 4d85c9               	test	r9, r9
;;   79:	 0f8519000000         	jne	0x98
;;   7f:	 4883ec08             	sub	rsp, 8
;;   83:	 4c89f7               	mov	rdi, r14
;;   86:	 be00000000           	mov	esi, 0
;;   8b:	 89da                 	mov	edx, ebx
;;   8d:	 ffd1                 	call	rcx
;;   8f:	 4883c408             	add	rsp, 8
;;   93:	 e904000000           	jmp	0x9c
;;   98:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   9c:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;   a0:	 418b0b               	mov	ecx, dword ptr [r11]
;;   a3:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;   a6:	 39d1                 	cmp	ecx, edx
;;   a8:	 0f85a0000000         	jne	0x14e
;;   ae:	 488b4810             	mov	rcx, qword ptr [rax + 0x10]
;;   b2:	 4883ec08             	sub	rsp, 8
;;   b6:	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;   ba:	 ffd1                 	call	rcx
;;   bc:	 4883c410             	add	rsp, 0x10
;;   c0:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   c4:	 83e901               	sub	ecx, 1
;;   c7:	 50                   	push	rax
;;   c8:	 51                   	push	rcx
;;   c9:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   cd:	 498b4b48             	mov	rcx, qword ptr [r11 + 0x48]
;;   d1:	 bb00000000           	mov	ebx, 0
;;   d6:	 4d89f1               	mov	r9, r14
;;   d9:	 4d8b4150             	mov	r8, qword ptr [r9 + 0x50]
;;   dd:	 4439c3               	cmp	ebx, r8d
;;   e0:	 0f836a000000         	jae	0x150
;;   e6:	 4189db               	mov	r11d, ebx
;;   e9:	 4d6bdb08             	imul	r11, r11, 8
;;   ed:	 4d8b4948             	mov	r9, qword ptr [r9 + 0x48]
;;   f1:	 4d89ca               	mov	r10, r9
;;   f4:	 4d01d9               	add	r9, r11
;;   f7:	 4439c3               	cmp	ebx, r8d
;;   fa:	 4d0f43ca             	cmovae	r9, r10
;;   fe:	 4d8b09               	mov	r9, qword ptr [r9]
;;  101:	 4c89c8               	mov	rax, r9
;;  104:	 4d85c9               	test	r9, r9
;;  107:	 0f8511000000         	jne	0x11e
;;  10d:	 4c89f7               	mov	rdi, r14
;;  110:	 be00000000           	mov	esi, 0
;;  115:	 89da                 	mov	edx, ebx
;;  117:	 ffd1                 	call	rcx
;;  119:	 e904000000           	jmp	0x122
;;  11e:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;  122:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;  126:	 418b0b               	mov	ecx, dword ptr [r11]
;;  129:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;  12c:	 39d1                 	cmp	ecx, edx
;;  12e:	 0f851e000000         	jne	0x152
;;  134:	 488b4810             	mov	rcx, qword ptr [rax + 0x10]
;;  138:	 8b3c24               	mov	edi, dword ptr [rsp]
;;  13b:	 ffd1                 	call	rcx
;;  13d:	 4883c408             	add	rsp, 8
;;  141:	 59                   	pop	rcx
;;  142:	 01c1                 	add	ecx, eax
;;  144:	 89c8                 	mov	eax, ecx
;;  146:	 4883c410             	add	rsp, 0x10
;;  14a:	 5d                   	pop	rbp
;;  14b:	 c3                   	ret	
;;  14c:	 0f0b                 	ud2	
;;  14e:	 0f0b                 	ud2	
;;  150:	 0f0b                 	ud2	
;;  152:	 0f0b                 	ud2	
