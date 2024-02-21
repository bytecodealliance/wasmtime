;;! target = "x86_64"
;;! flags = ["has_sse41"]

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.trunc)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8726000000         	ja	0x44
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 f30f11442404         	movss	dword ptr [rsp + 4], xmm0
;;      	 f30f10442404         	movss	xmm0, dword ptr [rsp + 4]
;;      	 660f3a0ac003         	roundss	xmm0, xmm0, 3
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   44:	 0f0b                 	ud2	
