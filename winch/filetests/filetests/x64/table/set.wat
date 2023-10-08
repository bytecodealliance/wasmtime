;;! target = "x86_64"


(module
  (table $t3 2 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)

  (func (export "set-funcref") (param $i i32) (param $r funcref)
    (table.set $t3 (local.get $i) (local.get $r))
  )
  (func (export "set-funcref-from") (param $i i32) (param $j i32)
    (table.set $t3 (local.get $i) (table.get $t3 (local.get $j)))
  )
)

;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 897c2414             	mov	dword ptr [rsp + 0x14], edi
;;    c:	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   1a:	 8b4c2414             	mov	ecx, dword ptr [rsp + 0x14]
;;   1e:	 4c89f2               	mov	rdx, r14
;;   21:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   24:	 39d9                 	cmp	ecx, ebx
;;   26:	 0f8324000000         	jae	0x50
;;   2c:	 4189cb               	mov	r11d, ecx
;;   2f:	 4d6bdb08             	imul	r11, r11, 8
;;   33:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   37:	 4889d6               	mov	rsi, rdx
;;   3a:	 4c01da               	add	rdx, r11
;;   3d:	 39d9                 	cmp	ecx, ebx
;;   3f:	 480f43d6             	cmovae	rdx, rsi
;;   43:	 4883c801             	or	rax, 1
;;   47:	 488902               	mov	qword ptr [rdx], rax
;;   4a:	 4883c418             	add	rsp, 0x18
;;   4e:	 5d                   	pop	rbp
;;   4f:	 c3                   	ret	
;;   50:	 0f0b                 	ud2	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   19:	 4153                 	push	r11
;;   1b:	 448b5c2410           	mov	r11d, dword ptr [rsp + 0x10]
;;   20:	 4153                 	push	r11
;;   22:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   26:	 498b4b48             	mov	rcx, qword ptr [r11 + 0x48]
;;   2a:	 5b                   	pop	rbx
;;   2b:	 4d89f1               	mov	r9, r14
;;   2e:	 458b5150             	mov	r10d, dword ptr [r9 + 0x50]
;;   32:	 4439d3               	cmp	ebx, r10d
;;   35:	 0f8377000000         	jae	0xb2
;;   3b:	 4189db               	mov	r11d, ebx
;;   3e:	 4d6bdb08             	imul	r11, r11, 8
;;   42:	 4d8b4948             	mov	r9, qword ptr [r9 + 0x48]
;;   46:	 4d89cc               	mov	r12, r9
;;   49:	 4d01d9               	add	r9, r11
;;   4c:	 4439d3               	cmp	ebx, r10d
;;   4f:	 4d0f43cc             	cmovae	r9, r12
;;   53:	 4d8b01               	mov	r8, qword ptr [r9]
;;   56:	 4c89c0               	mov	rax, r8
;;   59:	 4d85c0               	test	r8, r8
;;   5c:	 0f8519000000         	jne	0x7b
;;   62:	 4883ec08             	sub	rsp, 8
;;   66:	 4c89f7               	mov	rdi, r14
;;   69:	 be00000000           	mov	esi, 0
;;   6e:	 89da                 	mov	edx, ebx
;;   70:	 ffd1                 	call	rcx
;;   72:	 4883c408             	add	rsp, 8
;;   76:	 e904000000           	jmp	0x7f
;;   7b:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   7f:	 59                   	pop	rcx
;;   80:	 4c89f2               	mov	rdx, r14
;;   83:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   86:	 39d9                 	cmp	ecx, ebx
;;   88:	 0f8326000000         	jae	0xb4
;;   8e:	 4189cb               	mov	r11d, ecx
;;   91:	 4d6bdb08             	imul	r11, r11, 8
;;   95:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   99:	 4889d6               	mov	rsi, rdx
;;   9c:	 4c01da               	add	rdx, r11
;;   9f:	 39d9                 	cmp	ecx, ebx
;;   a1:	 480f43d6             	cmovae	rdx, rsi
;;   a5:	 4883c801             	or	rax, 1
;;   a9:	 488902               	mov	qword ptr [rdx], rax
;;   ac:	 4883c410             	add	rsp, 0x10
;;   b0:	 5d                   	pop	rbp
;;   b1:	 c3                   	ret	
;;   b2:	 0f0b                 	ud2	
;;   b4:	 0f0b                 	ud2	
