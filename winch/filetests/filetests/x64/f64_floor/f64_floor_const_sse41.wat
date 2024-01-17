;;! target = "x86_64"
;;! flags = ["has_sse41"]

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.floor)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10050c000000     	movsd	xmm0, qword ptr [rip + 0xc]
;;      	 660f3a0bc001         	roundsd	xmm0, xmm0, 1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
