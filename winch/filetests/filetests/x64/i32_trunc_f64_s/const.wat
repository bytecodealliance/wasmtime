;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100554000000     	movsd	xmm0, qword ptr [rip + 0x54]
;;      	 f20f2cc0             	cvttsd2si	eax, xmm0
;;      	 83f801               	cmp	eax, 1
;;      	 0f8134000000         	jno	0x55
;;   21:	 660f2ec0             	ucomisd	xmm0, xmm0
;;      	 0f8a30000000         	jp	0x5b
;;   2b:	 49bb000020000000e0c1 	
;; 				movabs	r11, 0xc1e0000000200000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ec7           	ucomisd	xmm0, xmm15
;;      	 0f8618000000         	jbe	0x5d
;;   45:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 66440f2ef8           	ucomisd	xmm15, xmm0
;;      	 0f820a000000         	jb	0x5f
;;   55:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5b:	 0f0b                 	ud2	
;;   5d:	 0f0b                 	ud2	
;;   5f:	 0f0b                 	ud2	
;;   61:	 0000                 	add	byte ptr [rax], al
;;   63:	 0000                 	add	byte ptr [rax], al
;;   65:	 0000                 	add	byte ptr [rax], al
;;   67:	 0000                 	add	byte ptr [rax], al
;;   69:	 0000                 	add	byte ptr [rax], al
;;   6b:	 0000                 	add	byte ptr [rax], al
;;   6d:	 00f0                 	add	al, dh
