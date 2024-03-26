;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8764000000         	ja	0x7f
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 f20f110424           	movsd	%xmm0, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 f2480f2cc0           	cvttsd2si	%xmm0, %rax
;;      	 4883f801             	cmpq	$1, %rax
;;      	 0f8134000000         	jno	0x79
;;   45:	 660f2ec0             	ucomisd	%xmm0, %xmm0
;;      	 0f8a32000000         	jp	0x81
;;   4f:	 49bb000000000000e0c3 	
;; 				movabsq	$14114281232179134464, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f2ec7           	ucomisd	%xmm15, %xmm0
;;      	 0f821a000000         	jb	0x83
;;   69:	 66450f57ff           	xorpd	%xmm15, %xmm15
;;      	 66440f2ef8           	ucomisd	%xmm0, %xmm15
;;      	 0f820c000000         	jb	0x85
;;   79:	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   7f:	 0f0b                 	ud2	
;;   81:	 0f0b                 	ud2	
;;   83:	 0f0b                 	ud2	
;;   85:	 0f0b                 	ud2	
