;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const -1.1)
        (f32.const 2.2)
        (f32.copysign)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c340000000       	add	r11, 0x40
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f877c000000         	ja	0x97
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
;;      	 f30f100550000000     	movss	xmm0, dword ptr [rip + 0x50]
;;      	 f30f100d50000000     	movss	xmm1, dword ptr [rip + 0x50]
;;      	 41bb00000080         	mov	r11d, 0x80000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f54c7             	andps	xmm0, xmm15
;;      	 440f55f9             	andnps	xmm15, xmm1
;;      	 410f28cf             	movaps	xmm1, xmm15
;;      	 0f56c8               	orps	xmm1, xmm0
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
;;   97:	 0f0b                 	ud2	
;;   99:	 0000                 	add	byte ptr [rax], al
;;   9b:	 0000                 	add	byte ptr [rax], al
;;   9d:	 0000                 	add	byte ptr [rax], al
;;   9f:	 00cd                 	add	ch, cl
;;   a1:	 cc                   	int3	
;;   a2:	 0c40                 	or	al, 0x40
;;   a4:	 0000                 	add	byte ptr [rax], al
;;   a6:	 0000                 	add	byte ptr [rax], al
;;   a8:	 cdcc                 	int	0xcc
