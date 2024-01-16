;;! target = "x86_64"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;      	 4883f801             	cmp	rax, 1
;;      	 0f8134000000         	jno	0x5b
;;   27:	 660f2ec0             	ucomisd	xmm0, xmm0
;;      	 0f8a30000000         	jp	0x61
;;   31:	 49bb000000000000e0c3 	
;; 				movabs	r11, 0xc3e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ec7           	ucomisd	xmm0, xmm15
;;      	 0f8218000000         	jb	0x63
;;   4b:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 66440f2ef8           	ucomisd	xmm15, xmm0
;;      	 0f820a000000         	jb	0x65
;;   5b:	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   61:	 0f0b                 	ud2	
;;   63:	 0f0b                 	ud2	
;;   65:	 0f0b                 	ud2	
