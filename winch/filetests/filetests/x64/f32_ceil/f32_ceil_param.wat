;;! target = "x86_64"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.ceil)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8738000000         	ja	0x50
;;   18:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f3440f107c240c       	movss	xmm15, dword ptr [rsp + 0xc]
;;      	 4883ec04             	sub	rsp, 4
;;      	 f3440f113c24         	movss	dword ptr [rsp], xmm15
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;      	 e800000000           	call	0x42
;;      	 4883c40c             	add	rsp, 0xc
;;      	 4883c404             	add	rsp, 4
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   50:	 0f0b                 	ud2	
