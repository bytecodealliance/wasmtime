;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const -1.1)
        (f64.const 2.2)
        (f64.copysign)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f100534000000     	movsd	xmm0, qword ptr [rip + 0x34]
;;   14:	 f20f100d34000000     	movsd	xmm1, qword ptr [rip + 0x34]
;;   1c:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   26:	 664d0f6efb           	movq	xmm15, r11
;;   2b:	 66410f54c7           	andpd	xmm0, xmm15
;;   30:	 66440f55f9           	andnpd	xmm15, xmm1
;;   35:	 66410f28cf           	movapd	xmm1, xmm15
;;   3a:	 660f56c8             	orpd	xmm1, xmm0
;;   3e:	 660f28c1             	movapd	xmm0, xmm1
;;   42:	 4883c408             	add	rsp, 8
;;   46:	 5d                   	pop	rbp
;;   47:	 c3                   	ret	
