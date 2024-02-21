;;! target = "x86_64"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.ceil)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8744000000         	ja	0x62
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f2440f103c24         	movsd	xmm15, qword ptr [rsp]
;;      	 4883ec08             	sub	rsp, 8
;;      	 f2440f113c24         	movsd	qword ptr [rsp], xmm15
;;      	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;      	 49bb0000000000000000 	
;; 				movabs	r11, 0
;;      	 41ffd3               	call	r11
;;      	 4883c408             	add	rsp, 8
;;      	 4c8b742410           	mov	r14, qword ptr [rsp + 0x10]
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   62:	 0f0b                 	ud2	
