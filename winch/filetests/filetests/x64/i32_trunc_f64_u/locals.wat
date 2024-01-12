;;! target = "x86_64"

(module
    (func (result i32)
        (local f64)  

        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f20f104c2408         	movsd	xmm1, qword ptr [rsp + 8]
;;   1b:	 49bb000000000000e041 	
;; 				movabs	r11, 0x41e0000000000000
;;   25:	 664d0f6efb           	movq	xmm15, r11
;;   2a:	 66410f2ecf           	ucomisd	xmm1, xmm15
;;   2f:	 0f8315000000         	jae	0x4a
;;   35:	 0f8a30000000         	jp	0x6b
;;   3b:	 f20f2cc1             	cvttsd2si	eax, xmm1
;;   3f:	 83f800               	cmp	eax, 0
;;   42:	 0f8d1d000000         	jge	0x65
;;   48:	 0f0b                 	ud2	
;;   4a:	 0f28c1               	movaps	xmm0, xmm1
;;   4d:	 f2410f5cc7           	subsd	xmm0, xmm15
;;   52:	 f20f2cc0             	cvttsd2si	eax, xmm0
;;   56:	 83f800               	cmp	eax, 0
;;   59:	 0f8c0e000000         	jl	0x6d
;;   5f:	 81c000000080         	add	eax, 0x80000000
;;   65:	 4883c410             	add	rsp, 0x10
;;   69:	 5d                   	pop	rbp
;;   6a:	 c3                   	ret	
;;   6b:	 0f0b                 	ud2	
;;   6d:	 0f0b                 	ud2	
