;;! target = "x86_64"

(module
  (func (export "main") (param i32) (param i32) (result i32)
    (local.get 1)
    (local.get 0)
    (i32.div_u)

    (call $add (i32.const 1) (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))

    (local.get 1)
    (local.get 0)
    (i32.div_u)

    (call $add (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))
  )

  (func $add (param i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.add)
    (local.get 2)
    (i32.add)
    (local.get 3)
    (i32.add)
    (local.get 4)
    (i32.add)
    (local.get 5)
    (i32.add)
    (local.get 6)
    (i32.add)
    (local.get 7)
    (i32.add)
    (local.get 8)
    (i32.add)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 897c2404             	mov	dword ptr [rsp + 4], edi
;;    c:	 893424               	mov	dword ptr [rsp], esi
;;    f:	 8b4c2404             	mov	ecx, dword ptr [rsp + 4]
;;   13:	 8b0424               	mov	eax, dword ptr [rsp]
;;   16:	 31d2                 	xor	edx, edx
;;   18:	 f7f1                 	div	ecx
;;   1a:	 50                   	push	rax
;;   1b:	 4883ec20             	sub	rsp, 0x20
;;   1f:	 8b7c2420             	mov	edi, dword ptr [rsp + 0x20]
;;   23:	 be01000000           	mov	esi, 1
;;   28:	 ba02000000           	mov	edx, 2
;;   2d:	 b903000000           	mov	ecx, 3
;;   32:	 41b804000000         	mov	r8d, 4
;;   38:	 41b905000000         	mov	r9d, 5
;;   3e:	 41bb06000000         	mov	r11d, 6
;;   44:	 44891c24             	mov	dword ptr [rsp], r11d
;;   48:	 41bb07000000         	mov	r11d, 7
;;   4e:	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;   53:	 41bb08000000         	mov	r11d, 8
;;   59:	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;   5e:	 e800000000           	call	0x63
;;   63:	 4883c428             	add	rsp, 0x28
;;   67:	 50                   	push	rax
;;   68:	 448b5c2408           	mov	r11d, dword ptr [rsp + 8]
;;   6d:	 4153                 	push	r11
;;   6f:	 448b5c2414           	mov	r11d, dword ptr [rsp + 0x14]
;;   74:	 4153                 	push	r11
;;   76:	 59                   	pop	rcx
;;   77:	 58                   	pop	rax
;;   78:	 31d2                 	xor	edx, edx
;;   7a:	 f7f1                 	div	ecx
;;   7c:	 50                   	push	rax
;;   7d:	 4883ec20             	sub	rsp, 0x20
;;   81:	 8b7c2428             	mov	edi, dword ptr [rsp + 0x28]
;;   85:	 8b742420             	mov	esi, dword ptr [rsp + 0x20]
;;   89:	 ba02000000           	mov	edx, 2
;;   8e:	 b903000000           	mov	ecx, 3
;;   93:	 41b804000000         	mov	r8d, 4
;;   99:	 41b905000000         	mov	r9d, 5
;;   9f:	 41bb06000000         	mov	r11d, 6
;;   a5:	 44891c24             	mov	dword ptr [rsp], r11d
;;   a9:	 41bb07000000         	mov	r11d, 7
;;   af:	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;   b4:	 41bb08000000         	mov	r11d, 8
;;   ba:	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;   bf:	 e800000000           	call	0xc4
;;   c4:	 4883c430             	add	rsp, 0x30
;;   c8:	 4883c408             	add	rsp, 8
;;   cc:	 5d                   	pop	rbp
;;   cd:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 897c2414             	mov	dword ptr [rsp + 0x14], edi
;;    c:	 89742410             	mov	dword ptr [rsp + 0x10], esi
;;   10:	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;   14:	 894c2408             	mov	dword ptr [rsp + 8], ecx
;;   18:	 4489442404           	mov	dword ptr [rsp + 4], r8d
;;   1d:	 44890c24             	mov	dword ptr [rsp], r9d
;;   21:	 8b442410             	mov	eax, dword ptr [rsp + 0x10]
;;   25:	 8b4c2414             	mov	ecx, dword ptr [rsp + 0x14]
;;   29:	 01c1                 	add	ecx, eax
;;   2b:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   2f:	 01c1                 	add	ecx, eax
;;   31:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   35:	 01c1                 	add	ecx, eax
;;   37:	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;   3b:	 01c1                 	add	ecx, eax
;;   3d:	 8b0424               	mov	eax, dword ptr [rsp]
;;   40:	 01c1                 	add	ecx, eax
;;   42:	 8b4510               	mov	eax, dword ptr [rbp + 0x10]
;;   45:	 01c1                 	add	ecx, eax
;;   47:	 8b4518               	mov	eax, dword ptr [rbp + 0x18]
;;   4a:	 01c1                 	add	ecx, eax
;;   4c:	 8b4520               	mov	eax, dword ptr [rbp + 0x20]
;;   4f:	 01c1                 	add	ecx, eax
;;   51:	 4889c8               	mov	rax, rcx
;;   54:	 4883c418             	add	rsp, 0x18
;;   58:	 5d                   	pop	rbp
;;   59:	 c3                   	ret	
