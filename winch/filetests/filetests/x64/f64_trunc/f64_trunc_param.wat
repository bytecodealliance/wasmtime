;;! target = "x86_64"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.trunc)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f2440f107c2408       	movsd	xmm15, qword ptr [rsp + 8]
;;      	 4883ec08             	sub	rsp, 8
;;      	 f2440f113c24         	movsd	qword ptr [rsp], xmm15
;;      	 4883ec08             	sub	rsp, 8
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 e800000000           	call	0x32
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
