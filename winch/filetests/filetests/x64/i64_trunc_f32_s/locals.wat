;;! target = "x86_64"

(module
    (func (result i64)
        (local f32)  

        (local.get 0)
        (i64.trunc_f32_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;   1b:	 f3480f2cc0           	cvttss2si	rax, xmm0
;;   20:	 4883f801             	cmp	rax, 1
;;   24:	 0f812d000000         	jno	0x57
;;   2a:	 0f2ec0               	ucomiss	xmm0, xmm0
;;   2d:	 0f8a2a000000         	jp	0x5d
;;   33:	 41bb000000df         	mov	r11d, 0xdf000000
;;   39:	 66450f6efb           	movd	xmm15, r11d
;;   3e:	 410f2ec7             	ucomiss	xmm0, xmm15
;;   42:	 0f8217000000         	jb	0x5f
;;   48:	 66450f57ff           	xorpd	xmm15, xmm15
;;   4d:	 440f2ef8             	ucomiss	xmm15, xmm0
;;   51:	 0f820a000000         	jb	0x61
;;   57:	 4883c410             	add	rsp, 0x10
;;   5b:	 5d                   	pop	rbp
;;   5c:	 c3                   	ret	
;;   5d:	 0f0b                 	ud2	
;;   5f:	 0f0b                 	ud2	
;;   61:	 0f0b                 	ud2	
