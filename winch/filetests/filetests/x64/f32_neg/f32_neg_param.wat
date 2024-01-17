;;! target = "x86_64"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.neg)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8725000000         	ja	0x3d
;;   18:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;      	 41bb00000080         	mov	r11d, 0x80000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f57c7             	xorps	xmm0, xmm15
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3d:	 0f0b                 	ud2	
