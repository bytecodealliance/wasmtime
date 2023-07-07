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
;;   20:	 50                   	push	rax
;;   21:	 4883ec28             	sub	rsp, 0x28
;;   25:	 8b7c2428             	mov	edi, dword ptr [rsp + 0x28]
;;   29:	 be01000000           	mov	esi, 1
;;   2e:	 ba02000000           	mov	edx, 2
;;   33:	 b903000000           	mov	ecx, 3
;;   38:	 41b804000000         	mov	r8d, 4
;;   3e:	 41b905000000         	mov	r9d, 5
;;   44:	 41bb06000000         	mov	r11d, 6
;;   4a:	 44891c24             	mov	dword ptr [rsp], r11d
;;   4e:	 41bb07000000         	mov	r11d, 7
;;   54:	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;   59:	 41bb08000000         	mov	r11d, 8
;;   5f:	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;   64:	 e800000000           	call	0x69
;;   69:	 4883c430             	add	rsp, 0x30
;;   6d:	 50                   	push	rax
;;   6e:	 448b5c2410           	mov	r11d, dword ptr [rsp + 0x10]
;;   73:	 4153                 	push	r11
;;   75:	 448b5c241c           	mov	r11d, dword ptr [rsp + 0x1c]
;;   7a:	 4153                 	push	r11
;;   7c:	 59                   	pop	rcx
;;   7d:	 58                   	pop	rax
;;   7e:	 31d2                 	xor	edx, edx
;;   80:	 f7f1                 	div	ecx
;;   82:	 50                   	push	rax
;;   83:	 4883ec20             	sub	rsp, 0x20
;;   87:	 8b7c2428             	mov	edi, dword ptr [rsp + 0x28]
;;   8b:	 8b742420             	mov	esi, dword ptr [rsp + 0x20]
;;   8f:	 ba02000000           	mov	edx, 2
;;   94:	 b903000000           	mov	ecx, 3
;;   99:	 41b804000000         	mov	r8d, 4
;;   9f:	 41b905000000         	mov	r9d, 5
;;   a5:	 41bb06000000         	mov	r11d, 6
;;   ab:	 44891c24             	mov	dword ptr [rsp], r11d
;;   af:	 41bb07000000         	mov	r11d, 7
;;   b5:	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;   ba:	 41bb08000000         	mov	r11d, 8
;;   c0:	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;   c5:	 e800000000           	call	0xca
;;   ca:	 4883c430             	add	rsp, 0x30
;;   ce:	 4883c410             	add	rsp, 0x10
;;   d2:	 5d                   	pop	rbp
;;   d3:	 c3                   	ret	
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
;;   57:	 4889c8               	mov	rax, rcx
;;   5a:	 4883c420             	add	rsp, 0x20
;;   5e:	 5d                   	pop	rbp
;;   5f:	 c3                   	ret	
