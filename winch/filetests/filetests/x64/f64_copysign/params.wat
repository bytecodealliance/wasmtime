;;! target = "x86_64"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.copysign)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;    e:	 f20f114c2408         	movsd	qword ptr [rsp + 8], xmm1
;;   14:	 4c893424             	mov	qword ptr [rsp], r14
;;   18:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   1e:	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;   24:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   2e:	 664d0f6efb           	movq	xmm15, r11
;;   33:	 66410f54c7           	andpd	xmm0, xmm15
;;   38:	 66440f55f9           	andnpd	xmm15, xmm1
;;   3d:	 66410f28cf           	movapd	xmm1, xmm15
;;   42:	 660f56c8             	orpd	xmm1, xmm0
;;   46:	 660f28c1             	movapd	xmm0, xmm1
;;   4a:	 4883c418             	add	rsp, 0x18
;;   4e:	 5d                   	pop	rbp
;;   4f:	 c3                   	ret	
