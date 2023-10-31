;;! target = "x86_64"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.sub)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 f30f114c2408         	movss	dword ptr [rsp + 8], xmm1
;;   14:	 4c893424             	mov	qword ptr [rsp], r14
;;   18:	 f30f10442408         	movss	xmm0, dword ptr [rsp + 8]
;;   1e:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   24:	 f30f5cc8             	subss	xmm1, xmm0
;;   28:	 0f28c1               	movaps	xmm0, xmm1
;;   2b:	 4883c410             	add	rsp, 0x10
;;   2f:	 5d                   	pop	rbp
;;   30:	 c3                   	ret	
