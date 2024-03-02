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
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c380000000       	add	r11, 0x80
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8771010000         	ja	0x18c
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2440           	mov	qword ptr [rsp + 0x40], rdi
;;      	 4889742438           	mov	qword ptr [rsp + 0x38], rsi
;;      	 89542434             	mov	dword ptr [rsp + 0x34], edx
;;      	 894c2430             	mov	dword ptr [rsp + 0x30], ecx
;;      	 8b4c2434             	mov	ecx, dword ptr [rsp + 0x34]
;;      	 8b442430             	mov	eax, dword ptr [rsp + 0x30]
;;      	 31d2                 	xor	edx, edx
;;      	 f7f1                 	div	ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec34             	sub	rsp, 0x34
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 8b542434             	mov	edx, dword ptr [rsp + 0x34]
;;      	 b901000000           	mov	ecx, 1
;;      	 41b802000000         	mov	r8d, 2
;;      	 41b903000000         	mov	r9d, 3
;;      	 41bb04000000         	mov	r11d, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 41bb05000000         	mov	r11d, 5
;;      	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;      	 41bb06000000         	mov	r11d, 6
;;      	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;      	 41bb07000000         	mov	r11d, 7
;;      	 44895c2418           	mov	dword ptr [rsp + 0x18], r11d
;;      	 41bb08000000         	mov	r11d, 8
;;      	 44895c2420           	mov	dword ptr [rsp + 0x20], r11d
;;      	 e800000000           	call	0xbd
;;      	 4883c434             	add	rsp, 0x34
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742440           	mov	r14, qword ptr [rsp + 0x40]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 448b5c2434           	mov	r11d, dword ptr [rsp + 0x34]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 448b5c243c           	mov	r11d, dword ptr [rsp + 0x3c]
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
;;      	 4883ec30             	sub	rsp, 0x30
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 8b542434             	mov	edx, dword ptr [rsp + 0x34]
;;      	 8b4c2430             	mov	ecx, dword ptr [rsp + 0x30]
;;      	 41b802000000         	mov	r8d, 2
;;      	 41b903000000         	mov	r9d, 3
;;      	 41bb04000000         	mov	r11d, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 41bb05000000         	mov	r11d, 5
;;      	 44895c2408           	mov	dword ptr [rsp + 8], r11d
;;      	 41bb06000000         	mov	r11d, 6
;;      	 44895c2410           	mov	dword ptr [rsp + 0x10], r11d
;;      	 41bb07000000         	mov	r11d, 7
;;      	 44895c2418           	mov	dword ptr [rsp + 0x18], r11d
;;      	 41bb08000000         	mov	r11d, 8
;;      	 44895c2420           	mov	dword ptr [rsp + 0x20], r11d
;;      	 e800000000           	call	0x15d
;;      	 4883c430             	add	rsp, 0x30
;;      	 4883c408             	add	rsp, 8
;;      	 4c8b742440           	mov	r14, qword ptr [rsp + 0x40]
;;      	 4883c418             	add	rsp, 0x18
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  18c:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c350000000       	add	r11, 0x50
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8792000000         	ja	0xad
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2448           	mov	qword ptr [rsp + 0x48], rdi
;;      	 4889742440           	mov	qword ptr [rsp + 0x40], rsi
;;      	 8954243c             	mov	dword ptr [rsp + 0x3c], edx
;;      	 894c2438             	mov	dword ptr [rsp + 0x38], ecx
;;      	 4489442434           	mov	dword ptr [rsp + 0x34], r8d
;;      	 44894c2430           	mov	dword ptr [rsp + 0x30], r9d
;;      	 8b442438             	mov	eax, dword ptr [rsp + 0x38]
;;      	 8b4c243c             	mov	ecx, dword ptr [rsp + 0x3c]
;;      	 01c1                 	add	ecx, eax
;;      	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 01c1                 	add	ecx, eax
;;      	 8b442430             	mov	eax, dword ptr [rsp + 0x30]
;;      	 01c1                 	add	ecx, eax
;;      	 8b4510               	mov	eax, dword ptr [rbp + 0x10]
;;      	 01c1                 	add	ecx, eax
;;      	 8b4518               	mov	eax, dword ptr [rbp + 0x18]
;;      	 01c1                 	add	ecx, eax
;;      	 8b4520               	mov	eax, dword ptr [rbp + 0x20]
;;      	 01c1                 	add	ecx, eax
;;      	 8b4528               	mov	eax, dword ptr [rbp + 0x28]
;;      	 01c1                 	add	ecx, eax
;;      	 8b4530               	mov	eax, dword ptr [rbp + 0x30]
;;      	 01c1                 	add	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c420             	add	rsp, 0x20
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   ad:	 0f0b                 	ud2	
