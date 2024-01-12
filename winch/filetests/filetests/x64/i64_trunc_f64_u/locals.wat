;;! target = "x86_64"

(module
    (func (result i64)
        (local f64)  

        (local.get 0)
        (i64.trunc_f64_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f20f104c2408         	movsd	xmm1, qword ptr [rsp + 8]
;;   1b:	 49bb000000000000e043 	
;; 				movabs	r11, 0x43e0000000000000
;;   25:	 664d0f6efb           	movq	xmm15, r11
;;   2a:	 66410f2ecf           	ucomisd	xmm1, xmm15
;;   2f:	 0f8317000000         	jae	0x4c
;;   35:	 0f8a3b000000         	jp	0x76
;;   3b:	 f2480f2cc1           	cvttsd2si	rax, xmm1
;;   40:	 4883f800             	cmp	rax, 0
;;   44:	 0f8d26000000         	jge	0x70
;;   4a:	 0f0b                 	ud2	
;;   4c:	 0f28c1               	movaps	xmm0, xmm1
;;   4f:	 f2410f5cc7           	subsd	xmm0, xmm15
;;   54:	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;   59:	 4883f800             	cmp	rax, 0
;;   5d:	 0f8c15000000         	jl	0x78
;;   63:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   6d:	 4c01d8               	add	rax, r11
;;   70:	 4883c410             	add	rsp, 0x10
;;   74:	 5d                   	pop	rbp
;;   75:	 c3                   	ret	
;;   76:	 0f0b                 	ud2	
;;   78:	 0f0b                 	ud2	
