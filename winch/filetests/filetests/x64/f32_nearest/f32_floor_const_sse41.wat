;;! target = "x86_64"
;;! flags = ["has_sse41"]

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.nearest)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f10050c000000     	movss	xmm0, dword ptr [rip + 0xc]
;;   14:	 660f3a0ac000         	roundss	xmm0, xmm0, 0
;;   1a:	 4883c408             	add	rsp, 8
;;   1e:	 5d                   	pop	rbp
;;   1f:	 c3                   	ret	
;;   20:	 c3                   	ret	
;;   21:	 f5                   	cmc	
;;   22:	 a8bf                 	test	al, 0xbf
