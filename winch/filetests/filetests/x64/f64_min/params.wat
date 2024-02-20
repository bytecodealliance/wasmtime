;;! target = "x86_64"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.min)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8751000000         	ja	0x6c
;;   1b:	 4883ec18             	sub	rsp, 0x18
;;      	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;      	 f20f114c2408         	movsd	qword ptr [rsp + 8], xmm1
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 0f8519000000         	jne	0x5e
;;      	 0f8a09000000         	jp	0x54
;;   4b:	 660f56c8             	orpd	xmm1, xmm0
;;      	 e90e000000           	jmp	0x62
;;   54:	 f20f58c8             	addsd	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x62
;;   5e:	 f20f5dc8             	minsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   6c:	 0f0b                 	ud2	
