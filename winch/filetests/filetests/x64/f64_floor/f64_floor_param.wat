;;! target = "x86_64"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.floor)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f2440f107c2408       	movsd	xmm15, qword ptr [rsp + 8]
;;   19:	 4883ec08             	sub	rsp, 8
;;   1d:	 f2440f113c24         	movsd	qword ptr [rsp], xmm15
;;   23:	 4883ec08             	sub	rsp, 8
;;   27:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   2d:	 e800000000           	call	0x32
;;   32:	 4883c408             	add	rsp, 8
;;   36:	 4883c408             	add	rsp, 8
;;   3a:	 4883c410             	add	rsp, 0x10
;;   3e:	 5d                   	pop	rbp
;;   3f:	 c3                   	ret	
