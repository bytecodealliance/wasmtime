;;! target = "x86_64"

(module
    (func (param f32) (result i32)
        (local.get 0)
        (i32.trunc_f32_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   18:	 41bb0000004f         	mov	r11d, 0x4f000000
;;   1e:	 66450f6efb           	movd	xmm15, r11d
;;   23:	 410f2ecf             	ucomiss	xmm1, xmm15
;;   27:	 0f8315000000         	jae	0x42
;;   2d:	 0f8a30000000         	jp	0x63
;;   33:	 f30f2cc1             	cvttss2si	eax, xmm1
;;   37:	 83f800               	cmp	eax, 0
;;   3a:	 0f8d1d000000         	jge	0x5d
;;   40:	 0f0b                 	ud2	
;;   42:	 0f28c1               	movaps	xmm0, xmm1
;;   45:	 f3410f5cc7           	subss	xmm0, xmm15
;;   4a:	 f30f2cc0             	cvttss2si	eax, xmm0
;;   4e:	 83f800               	cmp	eax, 0
;;   51:	 0f8c0e000000         	jl	0x65
;;   57:	 81c000000080         	add	eax, 0x80000000
;;   5d:	 4883c410             	add	rsp, 0x10
;;   61:	 5d                   	pop	rbp
;;   62:	 c3                   	ret	
;;   63:	 0f0b                 	ud2	
;;   65:	 0f0b                 	ud2	
