;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.le)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c340000000       	add	r11, 0x40
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f876b000000         	ja	0x86
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2438           	mov	qword ptr [rsp + 0x38], rdi
;;      	 4889742430           	mov	qword ptr [rsp + 0x30], rsi
;;      	 f30f100538000000     	movss	xmm0, dword ptr [rip + 0x38]
;;      	 f30f100d38000000     	movss	xmm1, dword ptr [rip + 0x38]
;;      	 0f2ec1               	ucomiss	xmm0, xmm1
;;      	 b800000000           	mov	eax, 0
;;      	 400f93c0             	setae	al
;;      	 4883c410             	add	rsp, 0x10
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   86:	 0f0b                 	ud2	
;;   88:	 cdcc                 	int	0xcc
;;   8a:	 0c40                 	or	al, 0x40
;;   8c:	 0000                 	add	byte ptr [rax], al
;;   8e:	 0000                 	add	byte ptr [rax], al
;;   90:	 cdcc                 	int	0xcc
