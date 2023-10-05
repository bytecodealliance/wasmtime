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
;;   2e:	 e925010000           	jmp	0x158
;;   33:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   37:	 83e802               	sub	eax, 2
;;   3a:	 50                   	push	rax
;;   3b:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   3f:	 498b4b48             	mov	rcx, qword ptr [r11 + 0x48]
;;   43:	 bb00000000           	mov	ebx, 0
;;   48:	 4d89f1               	mov	r9, r14
;;   4b:	 458b5150             	mov	r10d, dword ptr [r9 + 0x50]
;;   4f:	 4439d3               	cmp	ebx, r10d
;;   52:	 0f8306010000         	jae	0x15e
;;   58:	 4189db               	mov	r11d, ebx
;;   5b:	 4d6bdb08             	imul	r11, r11, 8
;;   5f:	 4d8b4948             	mov	r9, qword ptr [r9 + 0x48]
;;   63:	 4d89cc               	mov	r12, r9
;;   66:	 4d01d9               	add	r9, r11
;;   69:	 4439d3               	cmp	ebx, r10d
;;   6c:	 4d0f43cc             	cmovae	r9, r12
;;   70:	 4d8b01               	mov	r8, qword ptr [r9]
;;   73:	 4c89c0               	mov	rax, r8
;;   76:	 4d85c0               	test	r8, r8
;;   79:	 0f8519000000         	jne	0x98
;;   7f:	 4883ec08             	sub	rsp, 8
;;   83:	 4c89f7               	mov	rdi, r14
;;   86:	 be00000000           	mov	esi, 0
;;   8b:	 89da                 	mov	edx, ebx
;;   8d:	 ffd1                 	call	rcx
;;   8f:	 4883c408             	add	rsp, 8
;;   93:	 e904000000           	jmp	0x9c
;;   98:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   9c:	 4885c0               	test	rax, rax
;;   9f:	 0f84bb000000         	je	0x160
;;   a5:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;   a9:	 418b0b               	mov	ecx, dword ptr [r11]
;;   ac:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;   af:	 39d1                 	cmp	ecx, edx
;;   b1:	 0f85ab000000         	jne	0x162
;;   b7:	 488b4810             	mov	rcx, qword ptr [rax + 0x10]
;;   bb:	 4883ec08             	sub	rsp, 8
;;   bf:	 8b7c2408             	mov	edi, dword ptr [rsp + 8]
;;   c3:	 ffd1                 	call	rcx
;;   c5:	 4883c410             	add	rsp, 0x10
;;   c9:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   cd:	 83e901               	sub	ecx, 1
;;   d0:	 50                   	push	rax
;;   d1:	 51                   	push	rcx
;;   d2:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   d6:	 498b4b48             	mov	rcx, qword ptr [r11 + 0x48]
;;   da:	 bb00000000           	mov	ebx, 0
;;   df:	 4d89f1               	mov	r9, r14
;;   e2:	 458b5150             	mov	r10d, dword ptr [r9 + 0x50]
;;   e6:	 4439d3               	cmp	ebx, r10d
;;   e9:	 0f8375000000         	jae	0x164
;;   ef:	 4189db               	mov	r11d, ebx
;;   f2:	 4d6bdb08             	imul	r11, r11, 8
;;   f6:	 4d8b4948             	mov	r9, qword ptr [r9 + 0x48]
;;   fa:	 4d89cc               	mov	r12, r9
;;   fd:	 4d01d9               	add	r9, r11
;;  100:	 4439d3               	cmp	ebx, r10d
;;  103:	 4d0f43cc             	cmovae	r9, r12
;;  107:	 4d8b01               	mov	r8, qword ptr [r9]
;;  10a:	 4c89c0               	mov	rax, r8
;;  10d:	 4d85c0               	test	r8, r8
;;  110:	 0f8511000000         	jne	0x127
;;  116:	 4c89f7               	mov	rdi, r14
;;  119:	 be00000000           	mov	esi, 0
;;  11e:	 89da                 	mov	edx, ebx
;;  120:	 ffd1                 	call	rcx
;;  122:	 e904000000           	jmp	0x12b
;;  127:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;  12b:	 4885c0               	test	rax, rax
;;  12e:	 0f8432000000         	je	0x166
;;  134:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;  138:	 418b0b               	mov	ecx, dword ptr [r11]
;;  13b:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;  13e:	 39d1                 	cmp	ecx, edx
;;  140:	 0f8522000000         	jne	0x168
;;  146:	 488b4810             	mov	rcx, qword ptr [rax + 0x10]
;;  14a:	 8b3c24               	mov	edi, dword ptr [rsp]
;;  14d:	 ffd1                 	call	rcx
;;  14f:	 4883c408             	add	rsp, 8
;;  153:	 59                   	pop	rcx
;;  154:	 01c1                 	add	ecx, eax
;;  156:	 89c8                 	mov	eax, ecx
;;  158:	 4883c410             	add	rsp, 0x10
;;  15c:	 5d                   	pop	rbp
;;  15d:	 c3                   	ret	
;;  15e:	 0f0b                 	ud2	
;;  160:	 0f0b                 	ud2	
;;  162:	 0f0b                 	ud2	
;;  164:	 0f0b                 	ud2	
;;  166:	 0f0b                 	ud2	
;;  168:	 0f0b                 	ud2	
