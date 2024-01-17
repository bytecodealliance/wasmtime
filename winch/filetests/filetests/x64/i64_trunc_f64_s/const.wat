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
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100554000000     	movsd	xmm0, qword ptr [rip + 0x54]
;;      	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;      	 4883f801             	cmp	rax, 1
;;      	 0f8134000000         	jno	0x57
;;   23:	 660f2ec0             	ucomisd	xmm0, xmm0
;;      	 0f8a30000000         	jp	0x5d
;;   2d:	 49bb000000000000e0c3 	
;; 				movabs	r11, 0xc3e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ec7           	ucomisd	xmm0, xmm15
;;      	 0f8218000000         	jb	0x5f
;;   47:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 66440f2ef8           	ucomisd	xmm15, xmm0
;;      	 0f820a000000         	jb	0x61
;;   57:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5d:	 0f0b                 	ud2	
;;   5f:	 0f0b                 	ud2	
;;   61:	 0f0b                 	ud2	
;;   63:	 0000                 	add	byte ptr [rax], al
;;   65:	 0000                 	add	byte ptr [rax], al
;;   67:	 0000                 	add	byte ptr [rax], al
;;   69:	 0000                 	add	byte ptr [rax], al
;;   6b:	 0000                 	add	byte ptr [rax], al
;;   6d:	 00f0                 	add	al, dh
