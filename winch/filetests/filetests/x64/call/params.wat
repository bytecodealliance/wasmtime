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
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87fe000000         	ja	0x116
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 89742408             	mov	dword ptr [rsp + 8], esi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 31d2                 	xor	edx, edx
;;      	 f7f1                 	div	ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec2c             	sub	rsp, 0x2c
;;      	 8b7c242c             	mov	edi, dword ptr [rsp + 0x2c]
;;      	 be01000000           	mov	esi, 1
;;      	 ba02000000           	mov	edx, 2
;;      	 b903000000           	mov	ecx, 3
;;      	 41b804000000         	mov	r8d, 4
;;      	 41b905000000         	mov	r9d, 5
;;      	 41bb06000000         	mov	r11d, 6
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 41bb07000000         	mov	r11d, 7
;;      	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;      	 41bb08000000         	mov	r11d, 8
;;      	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;      	 e800000000           	call	0x7f
;;      	 4883c42c             	add	rsp, 0x2c
;;      	 4883c404             	add	rsp, 4
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 448b5c2414           	mov	r11d, dword ptr [rsp + 0x14]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 8b0424               	mov	eax, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 31d2                 	xor	edx, edx
;;      	 f7f1                 	div	ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec28             	sub	rsp, 0x28
;;      	 8b7c242c             	mov	edi, dword ptr [rsp + 0x2c]
;;      	 8b742428             	mov	esi, dword ptr [rsp + 0x28]
;;      	 ba02000000           	mov	edx, 2
;;      	 b903000000           	mov	ecx, 3
;;      	 41b804000000         	mov	r8d, 4
;;      	 41b905000000         	mov	r9d, 5
;;      	 41bb06000000         	mov	r11d, 6
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 41bb07000000         	mov	r11d, 7
;;      	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;      	 41bb08000000         	mov	r11d, 8
;;      	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;      	 e800000000           	call	0x108
;;      	 4883c428             	add	rsp, 0x28
;;      	 4883c408             	add	rsp, 8
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  116:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec20             	sub	rsp, 0x20
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8757000000         	ja	0x6f
;;   18:	 897c241c             	mov	dword ptr [rsp + 0x1c], edi
;;      	 89742418             	mov	dword ptr [rsp + 0x18], esi
;;      	 89542414             	mov	dword ptr [rsp + 0x14], edx
;;      	 894c2410             	mov	dword ptr [rsp + 0x10], ecx
;;      	 448944240c           	mov	dword ptr [rsp + 0xc], r8d
;;      	 44894c2408           	mov	dword ptr [rsp + 8], r9d
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b442418             	mov	eax, dword ptr [rsp + 0x18]
;;      	 8b4c241c             	mov	ecx, dword ptr [rsp + 0x1c]
;;      	 01c1                 	add	ecx, eax
;;      	 8b442414             	mov	eax, dword ptr [rsp + 0x14]
;;      	 01c1                 	add	ecx, eax
;;      	 8b442410             	mov	eax, dword ptr [rsp + 0x10]
;;      	 01c1                 	add	ecx, eax
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 01c1                 	add	ecx, eax
;;      	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;      	 01c1                 	add	ecx, eax
;;      	 8b4510               	mov	eax, dword ptr [rbp + 0x10]
;;      	 01c1                 	add	ecx, eax
;;      	 8b4518               	mov	eax, dword ptr [rbp + 0x18]
;;      	 01c1                 	add	ecx, eax
;;      	 8b4520               	mov	eax, dword ptr [rbp + 0x20]
;;      	 01c1                 	add	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   6f:	 0f0b                 	ud2	
