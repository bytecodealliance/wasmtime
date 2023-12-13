;;! target = "x86_64"

(module
    (func (param f64) (result i32)
        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f20f104c2408         	movsd	xmm1, qword ptr [rsp + 8]
;;   18:	 49bb000000000000e041 	
;; 				movabs	r11, 0x41e0000000000000
;;   22:	 664d0f6efb           	movq	xmm15, r11
;;   27:	 66410f2ecf           	ucomisd	xmm1, xmm15
;;   2c:	 0f8315000000         	jae	0x47
;;   32:	 0f8a30000000         	jp	0x68
;;   38:	 f20f2cc1             	cvttsd2si	eax, xmm1
;;   3c:	 83f800               	cmp	eax, 0
;;   3f:	 0f8d1d000000         	jge	0x62
;;   45:	 0f0b                 	ud2	
;;   47:	 0f28c1               	movaps	xmm0, xmm1
;;   4a:	 f2410f5cc7           	subsd	xmm0, xmm15
;;   4f:	 f20f2cc0             	cvttsd2si	eax, xmm0
;;   53:	 83f800               	cmp	eax, 0
;;   56:	 0f8c0e000000         	jl	0x6a
;;   5c:	 81c000000080         	add	eax, 0x80000000
;;   62:	 4883c410             	add	rsp, 0x10
;;   66:	 5d                   	pop	rbp
;;   67:	 c3                   	ret	
;;   68:	 0f0b                 	ud2	
;;   6a:	 0f0b                 	ud2	
