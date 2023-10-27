;;! target = "x86_64"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.floor)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   13:	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;   19:	 e800000000           	call	0x1e
;;   1e:	 4883c410             	add	rsp, 0x10
;;   22:	 5d                   	pop	rbp
;;   23:	 c3                   	ret	
