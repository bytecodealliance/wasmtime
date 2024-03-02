;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c340000000       	add	r11, 0x40
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8798000000         	ja	0xb3
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2438           	mov	qword ptr [rsp + 0x38], rdi
;;      	 4889742430           	mov	qword ptr [rsp + 0x30], rsi
;;      	 f20f100570000000     	movsd	xmm0, qword ptr [rip + 0x70]
;;      	 f20f2cc0             	cvttsd2si	eax, xmm0
;;      	 83f801               	cmp	eax, 1
;;      	 0f8134000000         	jno	0x91
;;   5d:	 660f2ec0             	ucomisd	xmm0, xmm0
;;      	 0f8a4e000000         	jp	0xb5
;;   67:	 49bb000020000000e0c1 	
;; 				movabs	r11, 0xc1e0000000200000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ec7           	ucomisd	xmm0, xmm15
;;      	 0f8636000000         	jbe	0xb7
;;   81:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 66440f2ef8           	ucomisd	xmm15, xmm0
;;      	 0f8228000000         	jb	0xb9
;;   91:	 4883c410             	add	rsp, 0x10
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   b3:	 0f0b                 	ud2	
;;   b5:	 0f0b                 	ud2	
;;   b7:	 0f0b                 	ud2	
;;   b9:	 0f0b                 	ud2	
;;   bb:	 0000                 	add	byte ptr [rax], al
;;   bd:	 0000                 	add	byte ptr [rax], al
;;   bf:	 0000                 	add	byte ptr [rax], al
;;   c1:	 0000                 	add	byte ptr [rax], al
;;   c3:	 0000                 	add	byte ptr [rax], al
;;   c5:	 00f0                 	add	al, dh
