;;! target = "x86_64"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8759000000         	ja	0x74
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100555000000     	movsd	xmm0, qword ptr [rip + 0x55]
;;      	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;      	 4883f801             	cmp	rax, 1
;;      	 0f8134000000         	jno	0x6e
;;   3a:	 660f2ec0             	ucomisd	xmm0, xmm0
;;      	 0f8a32000000         	jp	0x76
;;   44:	 49bb000000000000e0c3 	
;; 				movabs	r11, 0xc3e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ec7           	ucomisd	xmm0, xmm15
;;      	 0f821a000000         	jb	0x78
;;   5e:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 66440f2ef8           	ucomisd	xmm15, xmm0
;;      	 0f820c000000         	jb	0x7a
;;   6e:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   74:	 0f0b                 	ud2	
;;   76:	 0f0b                 	ud2	
;;   78:	 0f0b                 	ud2	
;;   7a:	 0f0b                 	ud2	
;;   7c:	 0000                 	add	byte ptr [rax], al
;;   7e:	 0000                 	add	byte ptr [rax], al
;;   80:	 0000                 	add	byte ptr [rax], al
;;   82:	 0000                 	add	byte ptr [rax], al
;;   84:	 0000                 	add	byte ptr [rax], al
