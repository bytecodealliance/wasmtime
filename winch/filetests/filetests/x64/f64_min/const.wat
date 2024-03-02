;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.min)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c340000000       	add	r11, 0x40
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f878a000000         	ja	0xa5
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
;;      	 f20f100558000000     	movsd	xmm0, qword ptr [rip + 0x58]
;;      	 f20f100d58000000     	movsd	xmm1, qword ptr [rip + 0x58]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 0f8519000000         	jne	0x7b
;;      	 0f8a09000000         	jp	0x71
;;   68:	 660f56c8             	orpd	xmm1, xmm0
;;      	 e90e000000           	jmp	0x7f
;;   71:	 f20f58c8             	addsd	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x7f
;;   7b:	 f20f5dc8             	minsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c410             	add	rsp, 0x10
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   a5:	 0f0b                 	ud2	
;;   a7:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   ad:	 99                   	cdq	
;;   ae:	 01409a               	add	dword ptr [rax - 0x66], eax
;;   b1:	 99                   	cdq	
;;   b2:	 99                   	cdq	
;;   b3:	 99                   	cdq	
;;   b4:	 99                   	cdq	
;;   b5:	 99                   	cdq	
;;   b6:	 f1                   	int1	
