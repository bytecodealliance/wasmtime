;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.ge
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c348000000       	add	r11, 0x48
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8799000000         	ja	0xb4
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
;;      	 48c744243000000000   	
;; 				mov	qword ptr [rsp + 0x30], 0
;;      	 f30f10055f000000     	movss	xmm0, dword ptr [rip + 0x5f]
;;      	 f30f11442434         	movss	dword ptr [rsp + 0x34], xmm0
;;      	 f30f100559000000     	movss	xmm0, dword ptr [rip + 0x59]
;;      	 f30f11442430         	movss	dword ptr [rsp + 0x30], xmm0
;;      	 f30f10442430         	movss	xmm0, dword ptr [rsp + 0x30]
;;      	 f30f104c2434         	movss	xmm1, dword ptr [rsp + 0x34]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 b800000000           	mov	eax, 0
;;      	 400f93c0             	setae	al
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f9bc3             	setnp	r11b
;;      	 4421d8               	and	eax, r11d
;;      	 4883c418             	add	rsp, 0x18
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   b4:	 0f0b                 	ud2	
;;   b6:	 0000                 	add	byte ptr [rax], al
;;   b8:	 cdcc                 	int	0xcc
