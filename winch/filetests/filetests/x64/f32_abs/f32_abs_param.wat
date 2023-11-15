;;! target = "x86_64"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.abs)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;   18:	 41bbffffff7f         	mov	r11d, 0x7fffffff
;;   1e:	 66450f6efb           	movd	xmm15, r11d
;;   23:	 410f54c7             	andps	xmm0, xmm15
;;   27:	 4883c410             	add	rsp, 0x10
;;   2b:	 5d                   	pop	rbp
;;   2c:	 c3                   	ret	
