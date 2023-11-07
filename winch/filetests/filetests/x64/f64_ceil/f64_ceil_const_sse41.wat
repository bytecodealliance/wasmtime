;;! target = "x86_64"
;;! flags = ["has_sse41"]

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.ceil)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10050c000000     	movsd	xmm0, qword ptr [rip + 0xc]
;;   14:	 660f3a0bc002         	roundsd	xmm0, xmm0, 2
;;   1a:	 4883c408             	add	rsp, 8
;;   1e:	 5d                   	pop	rbp
;;   1f:	 c3                   	ret	
