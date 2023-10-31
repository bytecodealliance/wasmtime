;;! target = "x86_64"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.min)
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
;;   24:	 0f2ec8               	ucomiss	xmm1, xmm0
;;   27:	 0f8518000000         	jne	0x45
;;   2d:	 0f8a08000000         	jp	0x3b
;;   33:	 0f56c8               	orps	xmm1, xmm0
;;   36:	 e90e000000           	jmp	0x49
;;   3b:	 f30f58c8             	addss	xmm1, xmm0
;;   3f:	 0f8a04000000         	jp	0x49
;;   45:	 f30f5dc8             	minss	xmm1, xmm0
;;   49:	 0f28c1               	movaps	xmm0, xmm1
;;   4c:	 4883c410             	add	rsp, 0x10
;;   50:	 5d                   	pop	rbp
;;   51:	 c3                   	ret	
