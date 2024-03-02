;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.min)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c340000000       	add	r11, 0x40
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8787000000         	ja	0xa2
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
;;      	 f30f100558000000     	movss	xmm0, dword ptr [rip + 0x58]
;;      	 f30f100d58000000     	movss	xmm1, dword ptr [rip + 0x58]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 0f8518000000         	jne	0x79
;;      	 0f8a08000000         	jp	0x6f
;;   67:	 0f56c8               	orps	xmm1, xmm0
;;      	 e90e000000           	jmp	0x7d
;;   6f:	 f30f58c8             	addss	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x7d
;;   79:	 f30f5dc8             	minss	xmm1, xmm0
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 4883c410             	add	rsp, 0x10
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   a2:	 0f0b                 	ud2	
;;   a4:	 0000                 	add	byte ptr [rax], al
;;   a6:	 0000                 	add	byte ptr [rax], al
;;   a8:	 cdcc                 	int	0xcc
;;   aa:	 0c40                 	or	al, 0x40
;;   ac:	 0000                 	add	byte ptr [rax], al
;;   ae:	 0000                 	add	byte ptr [rax], al
;;   b0:	 cdcc                 	int	0xcc
