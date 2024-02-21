;;! target = "x86_64"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8773000000         	ja	0x91
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100c24           	movsd	xmm1, qword ptr [rsp]
;;      	 49bb000000000000e043 	
;; 				movabs	r11, 0x43e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ecf           	ucomisd	xmm1, xmm15
;;      	 0f8317000000         	jae	0x67
;;      	 0f8a3d000000         	jp	0x93
;;   56:	 f2480f2cc1           	cvttsd2si	rax, xmm1
;;      	 4883f800             	cmp	rax, 0
;;      	 0f8d26000000         	jge	0x8b
;;   65:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f2410f5cc7           	subsd	xmm0, xmm15
;;      	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;      	 4883f800             	cmp	rax, 0
;;      	 0f8c17000000         	jl	0x95
;;   7e:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;      	 4c01d8               	add	rax, r11
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   91:	 0f0b                 	ud2	
;;   93:	 0f0b                 	ud2	
;;   95:	 0f0b                 	ud2	
