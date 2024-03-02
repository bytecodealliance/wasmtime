;;! target = "x86_64"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.reinterpret_f64)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c340000000       	add	r11, 0x40
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f875c000000         	ja	0x77
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
;;      	 f20f100530000000     	movsd	xmm0, qword ptr [rip + 0x30]
;;      	 66480f7ec0           	movq	rax, xmm0
;;      	 4883c410             	add	rsp, 0x10
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   77:	 0f0b                 	ud2	
;;   79:	 0000                 	add	byte ptr [rax], al
;;   7b:	 0000                 	add	byte ptr [rax], al
;;   7d:	 0000                 	add	byte ptr [rax], al
;;   7f:	 0000                 	add	byte ptr [rax], al
;;   81:	 0000                 	add	byte ptr [rax], al
;;   83:	 0000                 	add	byte ptr [rax], al
;;   85:	 00f0                 	add	al, dh
