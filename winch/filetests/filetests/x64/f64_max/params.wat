;;! target = "x86_64"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.max)
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
;;   24:	 660f2ec8             	ucomisd	xmm1, xmm0
;;   28:	 0f8519000000         	jne	0x47
;;   2e:	 0f8a09000000         	jp	0x3d
;;   34:	 660f54c8             	andpd	xmm1, xmm0
;;   38:	 e90e000000           	jmp	0x4b
;;   3d:	 f20f58c8             	addsd	xmm1, xmm0
;;   41:	 0f8a04000000         	jp	0x4b
;;   47:	 f20f5fc8             	maxsd	xmm1, xmm0
;;   4b:	 660f28c1             	movapd	xmm0, xmm1
;;   4f:	 4883c418             	add	rsp, 0x18
;;   53:	 5d                   	pop	rbp
;;   54:	 c3                   	ret	
