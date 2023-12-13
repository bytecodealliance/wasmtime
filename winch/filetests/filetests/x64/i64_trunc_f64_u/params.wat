;;! target = "x86_64"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f20f104c2408         	movsd	xmm1, qword ptr [rsp + 8]
;;   18:	 49bb000000000000e043 	
;; 				movabs	r11, 0x43e0000000000000
;;   22:	 664d0f6efb           	movq	xmm15, r11
;;   27:	 66410f2ecf           	ucomisd	xmm1, xmm15
;;   2c:	 0f8317000000         	jae	0x49
;;   32:	 0f8a3b000000         	jp	0x73
;;   38:	 f2480f2cc1           	cvttsd2si	rax, xmm1
;;   3d:	 4883f800             	cmp	rax, 0
;;   41:	 0f8d26000000         	jge	0x6d
;;   47:	 0f0b                 	ud2	
;;   49:	 0f28c1               	movaps	xmm0, xmm1
;;   4c:	 f2410f5cc7           	subsd	xmm0, xmm15
;;   51:	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;   56:	 4883f800             	cmp	rax, 0
;;   5a:	 0f8c15000000         	jl	0x75
;;   60:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   6a:	 4c01d8               	add	rax, r11
;;   6d:	 4883c410             	add	rsp, 0x10
;;   71:	 5d                   	pop	rbp
;;   72:	 c3                   	ret	
;;   73:	 0f0b                 	ud2	
;;   75:	 0f0b                 	ud2	
