;;! target = "x86_64"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8755000000         	ja	0x6d
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100554000000     	movsd	xmm0, qword ptr [rip + 0x54]
;;      	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;      	 4883f801             	cmp	rax, 1
;;      	 0f8134000000         	jno	0x67
;;   33:	 660f2ec0             	ucomisd	xmm0, xmm0
;;      	 0f8a32000000         	jp	0x6f
;;   3d:	 49bb000000000000e0c3 	
;; 				movabs	r11, 0xc3e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ec7           	ucomisd	xmm0, xmm15
;;      	 0f821a000000         	jb	0x71
;;   57:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 66440f2ef8           	ucomisd	xmm15, xmm0
;;      	 0f820c000000         	jb	0x73
;;   67:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   6d:	 0f0b                 	ud2	
;;   6f:	 0f0b                 	ud2	
;;   71:	 0f0b                 	ud2	
;;   73:	 0f0b                 	ud2	
;;   75:	 0000                 	add	byte ptr [rax], al
;;   77:	 0000                 	add	byte ptr [rax], al
;;   79:	 0000                 	add	byte ptr [rax], al
;;   7b:	 0000                 	add	byte ptr [rax], al
;;   7d:	 00f0                 	add	al, dh
