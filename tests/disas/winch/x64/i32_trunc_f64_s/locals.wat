;;! target = "x86_64"

(module
    (func (result i32)
        (local f64)  

        (local.get 0)
        (i32.trunc_f64_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8765000000         	ja	0x80
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 f20f2cc0             	cvttsd2si	%xmm0, %eax
;;      	 83f801               	cmpl	$1, %eax
;;      	 0f8134000000         	jno	0x7a
;;   46:	 660f2ec0             	ucomisd	%xmm0, %xmm0
;;      	 0f8a32000000         	jp	0x82
;;   50:	 49bb000020000000e0c1 	
;; 				movabsq	$13970166044105375744, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f2ec7           	ucomisd	%xmm15, %xmm0
;;      	 0f861a000000         	jbe	0x84
;;   6a:	 66450f57ff           	xorpd	%xmm15, %xmm15
;;      	 66440f2ef8           	ucomisd	%xmm0, %xmm15
;;      	 0f820c000000         	jb	0x86
;;   7a:	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   80:	 0f0b                 	ud2	
;;   82:	 0f0b                 	ud2	
;;   84:	 0f0b                 	ud2	
;;   86:	 0f0b                 	ud2	
