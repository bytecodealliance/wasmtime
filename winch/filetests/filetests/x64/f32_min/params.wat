;;! target = "x86_64"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.min)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f874a000000         	ja	0x62
;;   18:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 f30f114c2408         	movss	dword ptr [rsp + 8], xmm1
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10442408         	movss	xmm0, dword ptr [rsp + 8]
;;      	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 0f8518000000         	jne	0x55
;;      	 0f8a08000000         	jp	0x4b
;;   43:	 0f56c8               	orps	xmm1, xmm0
;;      	 e90e000000           	jmp	0x59
;;   4b:	 f30f58c8             	addss	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x59
;;   55:	 f30f5dc8             	minss	xmm1, xmm0
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   62:	 0f0b                 	ud2	
