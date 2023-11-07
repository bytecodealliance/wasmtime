;;! target = "x86_64"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.mul)
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
;;   24:	 f20f59c8             	mulsd	xmm1, xmm0
;;   28:	 660f28c1             	movapd	xmm0, xmm1
;;   2c:	 4883c418             	add	rsp, 0x18
;;   30:	 5d                   	pop	rbp
;;   31:	 c3                   	ret	
