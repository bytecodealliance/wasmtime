;;! target = "x86_64"

(module
    (func (param f64) (result i32)
        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f876b000000         	ja	0x86
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 f20f110424           	movsd	%xmm0, (%rsp)
;;      	 f20f100c24           	movsd	(%rsp), %xmm1
;;      	 49bb000000000000e041 	
;; 				movabsq	$0x41e0000000000000, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f2ecf           	ucomisd	%xmm15, %xmm1
;;      	 0f8315000000         	jae	0x65
;;      	 0f8a32000000         	jp	0x88
;;   56:	 f20f2cc1             	cvttsd2si	%xmm1, %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8d1d000000         	jge	0x80
;;   63:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 f2410f5cc7           	subsd	%xmm15, %xmm0
;;      	 f20f2cc0             	cvttsd2si	%xmm0, %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8c10000000         	jl	0x8a
;;   7a:	 81c000000080         	addl	$0x80000000, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   86:	 0f0b                 	ud2	
;;   88:	 0f0b                 	ud2	
;;   8a:	 0f0b                 	ud2	
