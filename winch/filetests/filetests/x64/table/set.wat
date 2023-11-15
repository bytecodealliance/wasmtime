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
;;   14:	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;   18:	 4c89f2               	mov	rdx, r14
;;   1b:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   1e:	 39d9                 	cmp	ecx, ebx
;;   20:	 0f838f000000         	jae	0xb5
;;   26:	 4189cb               	mov	r11d, ecx
;;   29:	 4d6bdb08             	imul	r11, r11, 8
;;   2d:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   31:	 4889d6               	mov	rsi, rdx
;;   34:	 4c01da               	add	rdx, r11
;;   37:	 39d9                 	cmp	ecx, ebx
;;   39:	 480f43d6             	cmovae	rdx, rsi
;;   3d:	 488b02               	mov	rax, qword ptr [rdx]
;;   40:	 4885c0               	test	rax, rax
;;   43:	 0f8532000000         	jne	0x7b
;;   49:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   4d:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   51:	 4156                 	push	r14
;;   53:	 4883ec04             	sub	rsp, 4
;;   57:	 890c24               	mov	dword ptr [rsp], ecx
;;   5a:	 4883ec04             	sub	rsp, 4
;;   5e:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;   63:	 be00000000           	mov	esi, 0
;;   68:	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;   6c:	 ffd3                 	call	rbx
;;   6e:	 4883c404             	add	rsp, 4
;;   72:	 4883c40c             	add	rsp, 0xc
;;   76:	 e904000000           	jmp	0x7f
;;   7b:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   7f:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   83:	 4c89f2               	mov	rdx, r14
;;   86:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   89:	 39d9                 	cmp	ecx, ebx
;;   8b:	 0f8326000000         	jae	0xb7
;;   91:	 4189cb               	mov	r11d, ecx
;;   94:	 4d6bdb08             	imul	r11, r11, 8
;;   98:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   9c:	 4889d6               	mov	rsi, rdx
;;   9f:	 4c01da               	add	rdx, r11
;;   a2:	 39d9                 	cmp	ecx, ebx
;;   a4:	 480f43d6             	cmovae	rdx, rsi
;;   a8:	 4883c801             	or	rax, 1
;;   ac:	 488902               	mov	qword ptr [rdx], rax
;;   af:	 4883c410             	add	rsp, 0x10
;;   b3:	 5d                   	pop	rbp
;;   b4:	 c3                   	ret	
;;   b5:	 0f0b                 	ud2	
;;   b7:	 0f0b                 	ud2	
