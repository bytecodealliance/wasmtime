;;! target = "x86_64"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   18:	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;   1d:	 4883f801             	cmp	rax, 1
;;   21:	 0f8134000000         	jno	0x5b
;;   27:	 660f2ec0             	ucomisd	xmm0, xmm0
;;   2b:	 0f8a30000000         	jp	0x61
;;   31:	 49bb000000000000e0c3 	
;; 				movabs	r11, 0xc3e0000000000000
;;   3b:	 664d0f6efb           	movq	xmm15, r11
;;   40:	 66410f2ec7           	ucomisd	xmm0, xmm15
;;   45:	 0f8218000000         	jb	0x63
;;   4b:	 66450f57ff           	xorpd	xmm15, xmm15
;;   50:	 66440f2ef8           	ucomisd	xmm15, xmm0
;;   55:	 0f820a000000         	jb	0x65
;;   5b:	 4883c410             	add	rsp, 0x10
;;   5f:	 5d                   	pop	rbp
;;   60:	 c3                   	ret	
;;   61:	 0f0b                 	ud2	
;;   63:	 0f0b                 	ud2	
;;   65:	 0f0b                 	ud2	
