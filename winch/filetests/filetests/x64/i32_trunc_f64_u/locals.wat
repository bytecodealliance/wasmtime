;;! target = "x86_64"

(module
    (func (result i32)
        (local f64)  

        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8763000000         	ja	0x7b
;;   18:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f104c2408         	movsd	xmm1, qword ptr [rsp + 8]
;;      	 49bb000000000000e041 	
;; 				movabs	r11, 0x41e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ecf           	ucomisd	xmm1, xmm15
;;      	 0f8315000000         	jae	0x5a
;;      	 0f8a32000000         	jp	0x7d
;;   4b:	 f20f2cc1             	cvttsd2si	eax, xmm1
;;      	 83f800               	cmp	eax, 0
;;      	 0f8d1d000000         	jge	0x75
;;   58:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f2410f5cc7           	subsd	xmm0, xmm15
;;      	 f20f2cc0             	cvttsd2si	eax, xmm0
;;      	 83f800               	cmp	eax, 0
;;      	 0f8c10000000         	jl	0x7f
;;   6f:	 81c000000080         	add	eax, 0x80000000
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   7b:	 0f0b                 	ud2	
;;   7d:	 0f0b                 	ud2	
;;   7f:	 0f0b                 	ud2	