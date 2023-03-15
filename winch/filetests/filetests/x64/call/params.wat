;;! target = "x86_64"

(module
  (func (export "main") (result i32)
    (local $x i32)
    (local $y i32)

    (local.set $x (i32.const 10))
    (local.set $y (i32.const 20))

    (local.get $y)
    (local.get $x)
    (i32.div_u)

    (call $add (i32.const 1) (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))
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
;;    8:	 48c7042400000000     	mov	qword ptr [rsp], 0
;;   10:	 b80a000000           	mov	eax, 0xa
;;   15:	 89442404             	mov	dword ptr [rsp + 4], eax
;;   19:	 b814000000           	mov	eax, 0x14
;;   1e:	 890424               	mov	dword ptr [rsp], eax
;;   21:	 8b4c2404             	mov	ecx, dword ptr [rsp + 4]
;;   25:	 8b0424               	mov	eax, dword ptr [rsp]
;;   28:	 31d2                 	xor	edx, edx
;;   2a:	 f7f1                 	div	ecx
;;   2c:	 50                   	push	rax
;;   2d:	 4883ec20             	sub	rsp, 0x20
;;   31:	 8b7c2410             	mov	edi, dword ptr [rsp + 0x10]
;;   35:	 be01000000           	mov	esi, 1
;;   3a:	 ba02000000           	mov	edx, 2
;;   3f:	 b903000000           	mov	ecx, 3
;;   44:	 41b804000000         	mov	r8d, 4
;;   4a:	 41b905000000         	mov	r9d, 5
;;   50:	 41bb06000000         	mov	r11d, 6
;;   56:	 44891c24             	mov	dword ptr [rsp], r11d
;;   5a:	 41bb07000000         	mov	r11d, 7
;;   60:	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;   65:	 41bb08000000         	mov	r11d, 8
;;   6b:	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;   70:	 e800000000           	call	0x75
;;   75:	 4883c428             	add	rsp, 0x28
;;   79:	 4883c408             	add	rsp, 8
;;   7d:	 5d                   	pop	rbp
;;   7e:	 c3                   	ret	
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
