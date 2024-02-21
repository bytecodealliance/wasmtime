;;! target = "x86_64"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.min)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8752000000         	ja	0x70
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 f30f11442404         	movss	dword ptr [rsp + 4], xmm0
;;      	 f30f110c24           	movss	dword ptr [rsp], xmm1
;;      	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;      	 f30f104c2404         	movss	xmm1, dword ptr [rsp + 4]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 0f8518000000         	jne	0x63
;;      	 0f8a08000000         	jp	0x59
;;   51:	 0f56c8               	orps	xmm1, xmm0
;;      	 e90e000000           	jmp	0x67
;;   59:	 f30f58c8             	addss	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x67
;;   63:	 f30f5dc8             	minss	xmm1, xmm0
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   70:	 0f0b                 	ud2	
