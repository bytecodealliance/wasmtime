;;! target = "x86_64"

(module
    (func (result i64)
        (local f64)  

        (local.get 0)
        (i64.trunc_f64_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8760000000         	ja	0x7b
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;      	 4883f801             	cmp	rax, 1
;;      	 0f8134000000         	jno	0x75
;;   41:	 660f2ec0             	ucomisd	xmm0, xmm0
;;      	 0f8a32000000         	jp	0x7d
;;   4b:	 49bb000000000000e0c3 	
;; 				movabs	r11, 0xc3e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ec7           	ucomisd	xmm0, xmm15
;;      	 0f821a000000         	jb	0x7f
;;   65:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 66440f2ef8           	ucomisd	xmm15, xmm0
;;      	 0f820c000000         	jb	0x81
;;   75:	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   7b:	 0f0b                 	ud2	
;;   7d:	 0f0b                 	ud2	
;;   7f:	 0f0b                 	ud2	
;;   81:	 0f0b                 	ud2	
