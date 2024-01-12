;;! target = "x86_64"

(module
    (func (result i32)
        (local f64)  

        (local.get 0)
        (i32.trunc_f64_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   1b:	 f20f2cc0             	cvttsd2si	eax, xmm0
;;   1f:	 83f801               	cmp	eax, 1
;;   22:	 0f8134000000         	jno	0x5c
;;   28:	 660f2ec0             	ucomisd	xmm0, xmm0
;;   2c:	 0f8a30000000         	jp	0x62
;;   32:	 49bb000020000000e0c1 	
;; 				movabs	r11, 0xc1e0000000200000
;;   3c:	 664d0f6efb           	movq	xmm15, r11
;;   41:	 66410f2ec7           	ucomisd	xmm0, xmm15
;;   46:	 0f8618000000         	jbe	0x64
;;   4c:	 66450f57ff           	xorpd	xmm15, xmm15
;;   51:	 66440f2ef8           	ucomisd	xmm15, xmm0
;;   56:	 0f820a000000         	jb	0x66
;;   5c:	 4883c410             	add	rsp, 0x10
;;   60:	 5d                   	pop	rbp
;;   61:	 c3                   	ret	
;;   62:	 0f0b                 	ud2	
;;   64:	 0f0b                 	ud2	
;;   66:	 0f0b                 	ud2	
