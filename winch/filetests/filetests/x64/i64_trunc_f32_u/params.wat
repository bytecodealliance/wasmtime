;;! target = "x86_64"

(module
    (func (param f32) (result i64)
        (local.get 0)
        (i64.trunc_f32_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   18:	 41bb0000005f         	mov	r11d, 0x5f000000
;;   1e:	 66450f6efb           	movd	xmm15, r11d
;;   23:	 410f2ecf             	ucomiss	xmm1, xmm15
;;   27:	 0f8317000000         	jae	0x44
;;   2d:	 0f8a3b000000         	jp	0x6e
;;   33:	 f3480f2cc1           	cvttss2si	rax, xmm1
;;   38:	 4883f800             	cmp	rax, 0
;;   3c:	 0f8d26000000         	jge	0x68
;;   42:	 0f0b                 	ud2	
;;   44:	 0f28c1               	movaps	xmm0, xmm1
;;   47:	 f3410f5cc7           	subss	xmm0, xmm15
;;   4c:	 f3480f2cc0           	cvttss2si	rax, xmm0
;;   51:	 4883f800             	cmp	rax, 0
;;   55:	 0f8c15000000         	jl	0x70
;;   5b:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   65:	 4c01d8               	add	rax, r11
;;   68:	 4883c410             	add	rsp, 0x10
;;   6c:	 5d                   	pop	rbp
;;   6d:	 c3                   	ret	
;;   6e:	 0f0b                 	ud2	
;;   70:	 0f0b                 	ud2	
