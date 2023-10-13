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
;;   20:	 0f8380000000         	jae	0xa6
;;   26:	 4189cb               	mov	r11d, ecx
;;   29:	 4d6bdb08             	imul	r11, r11, 8
;;   2d:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   31:	 4889d6               	mov	rsi, rdx
;;   34:	 4c01da               	add	rdx, r11
;;   37:	 39d9                 	cmp	ecx, ebx
;;   39:	 480f43d6             	cmovae	rdx, rsi
;;   3d:	 488b02               	mov	rax, qword ptr [rdx]
;;   40:	 4885c0               	test	rax, rax
;;   43:	 0f8523000000         	jne	0x6c
;;   49:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   4d:	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;   51:	 4156                 	push	r14
;;   53:	 51                   	push	rcx
;;   54:	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;   59:	 be00000000           	mov	esi, 0
;;   5e:	 8b1424               	mov	edx, dword ptr [rsp]
;;   61:	 ffd3                 	call	rbx
;;   63:	 4883c410             	add	rsp, 0x10
;;   67:	 e904000000           	jmp	0x70
;;   6c:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   70:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   74:	 4c89f2               	mov	rdx, r14
;;   77:	 8b5a50               	mov	ebx, dword ptr [rdx + 0x50]
;;   7a:	 39d9                 	cmp	ecx, ebx
;;   7c:	 0f8326000000         	jae	0xa8
;;   82:	 4189cb               	mov	r11d, ecx
;;   85:	 4d6bdb08             	imul	r11, r11, 8
;;   89:	 488b5248             	mov	rdx, qword ptr [rdx + 0x48]
;;   8d:	 4889d6               	mov	rsi, rdx
;;   90:	 4c01da               	add	rdx, r11
;;   93:	 39d9                 	cmp	ecx, ebx
;;   95:	 480f43d6             	cmovae	rdx, rsi
;;   99:	 4883c801             	or	rax, 1
;;   9d:	 488902               	mov	qword ptr [rdx], rax
;;   a0:	 4883c410             	add	rsp, 0x10
;;   a4:	 5d                   	pop	rbp
;;   a5:	 c3                   	ret	
;;   a6:	 0f0b                 	ud2	
;;   a8:	 0f0b                 	ud2	
