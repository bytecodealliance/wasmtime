;;! target = "x86_64"

(module
    (func (result i32)
        (local f32)  

        (local.get 0)
        (i32.trunc_f32_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;   1b:	 f30f2cc0             	cvttss2si	eax, xmm0
;;   1f:	 83f801               	cmp	eax, 1
;;   22:	 0f812d000000         	jno	0x55
;;   28:	 0f2ec0               	ucomiss	xmm0, xmm0
;;   2b:	 0f8a2a000000         	jp	0x5b
;;   31:	 41bb000000cf         	mov	r11d, 0xcf000000
;;   37:	 66450f6efb           	movd	xmm15, r11d
;;   3c:	 410f2ec7             	ucomiss	xmm0, xmm15
;;   40:	 0f8217000000         	jb	0x5d
;;   46:	 66450f57ff           	xorpd	xmm15, xmm15
;;   4b:	 440f2ef8             	ucomiss	xmm15, xmm0
;;   4f:	 0f820a000000         	jb	0x5f
;;   55:	 4883c410             	add	rsp, 0x10
;;   59:	 5d                   	pop	rbp
;;   5a:	 c3                   	ret	
;;   5b:	 0f0b                 	ud2	
;;   5d:	 0f0b                 	ud2	
;;   5f:	 0f0b                 	ud2	
