;;! target = "x86_64"
;;! flags = ["has_sse41"]

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.ceil)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   18:	 660f3a0bc002         	roundsd	xmm0, xmm0, 2
;;   1e:	 4883c410             	add	rsp, 0x10
;;   22:	 5d                   	pop	rbp
;;   23:	 c3                   	ret	
