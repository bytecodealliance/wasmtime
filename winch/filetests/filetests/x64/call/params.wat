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
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   18:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   1c:	 31d2                 	xor	edx, edx
;;   1e:	 f7f1                 	div	ecx
;;   20:	 4883ec04             	sub	rsp, 4
;;   24:	 890424               	mov	dword ptr [rsp], eax
;;   27:	 4883ec2c             	sub	rsp, 0x2c
;;   2b:	 8b7c242c             	mov	edi, dword ptr [rsp + 0x2c]
;;   2f:	 be01000000           	mov	esi, 1
;;   34:	 ba02000000           	mov	edx, 2
;;   39:	 b903000000           	mov	ecx, 3
;;   3e:	 41b804000000         	mov	r8d, 4
;;   44:	 41b905000000         	mov	r9d, 5
;;   4a:	 41bb06000000         	mov	r11d, 6
;;   50:	 44891c24             	mov	dword ptr [rsp], r11d
;;   54:	 41bb07000000         	mov	r11d, 7
;;   5a:	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;   5f:	 41bb08000000         	mov	r11d, 8
;;   65:	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;   6a:	 e800000000           	call	0x6f
;;   6f:	 4883c42c             	add	rsp, 0x2c
;;   73:	 4883c404             	add	rsp, 4
;;   77:	 4883ec04             	sub	rsp, 4
;;   7b:	 890424               	mov	dword ptr [rsp], eax
;;   7e:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   83:	 4883ec04             	sub	rsp, 4
;;   87:	 44891c24             	mov	dword ptr [rsp], r11d
;;   8b:	 448b5c2414           	mov	r11d, dword ptr [rsp + 0x14]
;;   90:	 4883ec04             	sub	rsp, 4
;;   94:	 44891c24             	mov	dword ptr [rsp], r11d
;;   98:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   9b:	 4883c404             	add	rsp, 4
;;   9f:	 8b0424               	mov	eax, dword ptr [rsp]
;;   a2:	 4883c404             	add	rsp, 4
;;   a6:	 31d2                 	xor	edx, edx
;;   a8:	 f7f1                 	div	ecx
;;   aa:	 4883ec04             	sub	rsp, 4
;;   ae:	 890424               	mov	dword ptr [rsp], eax
;;   b1:	 4883ec28             	sub	rsp, 0x28
;;   b5:	 8b7c242c             	mov	edi, dword ptr [rsp + 0x2c]
;;   b9:	 8b742428             	mov	esi, dword ptr [rsp + 0x28]
;;   bd:	 ba02000000           	mov	edx, 2
;;   c2:	 b903000000           	mov	ecx, 3
;;   c7:	 41b804000000         	mov	r8d, 4
;;   cd:	 41b905000000         	mov	r9d, 5
;;   d3:	 41bb06000000         	mov	r11d, 6
;;   d9:	 44891c24             	mov	dword ptr [rsp], r11d
;;   dd:	 41bb07000000         	mov	r11d, 7
;;   e3:	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;   e8:	 41bb08000000         	mov	r11d, 8
;;   ee:	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;   f3:	 e800000000           	call	0xf8
;;   f8:	 4883c428             	add	rsp, 0x28
;;   fc:	 4883c408             	add	rsp, 8
;;  100:	 4883c410             	add	rsp, 0x10
;;  104:	 5d                   	pop	rbp
;;  105:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec20             	sub	rsp, 0x20
;;    8:	 897c241c             	mov	dword ptr [rsp + 0x1c], edi
;;    c:	 89742418             	mov	dword ptr [rsp + 0x18], esi
;;   10:	 89542414             	mov	dword ptr [rsp + 0x14], edx
;;   14:	 894c2410             	mov	dword ptr [rsp + 0x10], ecx
;;   18:	 448944240c           	mov	dword ptr [rsp + 0xc], r8d
;;   1d:	 44894c2408           	mov	dword ptr [rsp + 8], r9d
;;   22:	 4c893424             	mov	qword ptr [rsp], r14
;;   26:	 8b442418             	mov	eax, dword ptr [rsp + 0x18]
;;   2a:	 8b4c241c             	mov	ecx, dword ptr [rsp + 0x1c]
;;   2e:	 01c1                 	add	ecx, eax
;;   30:	 8b442414             	mov	eax, dword ptr [rsp + 0x14]
;;   34:	 01c1                 	add	ecx, eax
;;   36:	 8b442410             	mov	eax, dword ptr [rsp + 0x10]
;;   3a:	 01c1                 	add	ecx, eax
;;   3c:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   40:	 01c1                 	add	ecx, eax
;;   42:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   46:	 01c1                 	add	ecx, eax
;;   48:	 8b4510               	mov	eax, dword ptr [rbp + 0x10]
;;   4b:	 01c1                 	add	ecx, eax
;;   4d:	 8b4518               	mov	eax, dword ptr [rbp + 0x18]
;;   50:	 01c1                 	add	ecx, eax
;;   52:	 8b4520               	mov	eax, dword ptr [rbp + 0x20]
;;   55:	 01c1                 	add	ecx, eax
;;   57:	 89c8                 	mov	eax, ecx
;;   59:	 4883c420             	add	rsp, 0x20
;;   5d:	 5d                   	pop	rbp
;;   5e:	 c3                   	ret	
