;;! target = "x86_64"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.neg)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   18:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   22:	 664d0f6efb           	movq	xmm15, r11
;;   27:	 66410f57c7           	xorpd	xmm0, xmm15
;;   2c:	 4883c410             	add	rsp, 0x10
;;   30:	 5d                   	pop	rbp
;;   31:	 c3                   	ret	
