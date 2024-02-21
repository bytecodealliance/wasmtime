;;! target = "x86_64"

(module
    (func (param f64) (result i32)
        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8768000000         	ja	0x86
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100c24           	movsd	xmm1, qword ptr [rsp]
;;      	 49bb000000000000e041 	
;; 				movabs	r11, 0x41e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ecf           	ucomisd	xmm1, xmm15
;;      	 0f8315000000         	jae	0x65
;;      	 0f8a32000000         	jp	0x88
;;   56:	 f20f2cc1             	cvttsd2si	eax, xmm1
;;      	 83f800               	cmp	eax, 0
;;      	 0f8d1d000000         	jge	0x80
;;   63:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f2410f5cc7           	subsd	xmm0, xmm15
;;      	 f20f2cc0             	cvttsd2si	eax, xmm0
;;      	 83f800               	cmp	eax, 0
;;      	 0f8c10000000         	jl	0x8a
;;   7a:	 81c000000080         	add	eax, 0x80000000
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   86:	 0f0b                 	ud2	
;;   88:	 0f0b                 	ud2	
;;   8a:	 0f0b                 	ud2	
