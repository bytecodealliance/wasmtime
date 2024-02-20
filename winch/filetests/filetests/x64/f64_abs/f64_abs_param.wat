;;! target = "x86_64"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.abs)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872e000000         	ja	0x49
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 49bbffffffffffffff7f 	
;; 				movabs	r11, 0x7fffffffffffffff
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f54c7           	andpd	xmm0, xmm15
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   49:	 0f0b                 	ud2	
