;;! target = "x86_64"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.sqrt)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;   18:	 f30f51c0             	sqrtss	xmm0, xmm0
;;   1c:	 4883c410             	add	rsp, 0x10
;;   20:	 5d                   	pop	rbp
;;   21:	 c3                   	ret	
