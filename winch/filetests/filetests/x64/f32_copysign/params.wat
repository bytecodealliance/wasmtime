;;! target = "x86_64"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.copysign)
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
;;   24:	 41bb00000080         	mov	r11d, 0x80000000
;;   2a:	 66450f6efb           	movd	xmm15, r11d
;;   2f:	 410f54c7             	andps	xmm0, xmm15
;;   33:	 440f55f9             	andnps	xmm15, xmm1
;;   37:	 410f28cf             	movaps	xmm1, xmm15
;;   3b:	 0f56c8               	orps	xmm1, xmm0
;;   3e:	 0f28c1               	movaps	xmm0, xmm1
;;   41:	 4883c410             	add	rsp, 0x10
;;   45:	 5d                   	pop	rbp
;;   46:	 c3                   	ret	
