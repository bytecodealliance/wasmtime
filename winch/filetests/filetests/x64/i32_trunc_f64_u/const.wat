;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8768000000         	ja	0x83
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f100d5d000000     	movsd	0x5d(%rip), %xmm1
;;      	 49bb000000000000e041 	
;; 				movabsq	$0x41e0000000000000, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f2ecf           	ucomisd	%xmm15, %xmm1
;;      	 0f8315000000         	jae	0x62
;;      	 0f8a32000000         	jp	0x85
;;   53:	 f20f2cc1             	cvttsd2si	%xmm1, %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8d1d000000         	jge	0x7d
;;   60:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 f2410f5cc7           	subsd	%xmm15, %xmm0
;;      	 f20f2cc0             	cvttsd2si	%xmm0, %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8c10000000         	jl	0x87
;;   77:	 81c000000080         	addl	$0x80000000, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   83:	 0f0b                 	ud2	
;;   85:	 0f0b                 	ud2	
;;   87:	 0f0b                 	ud2	
;;   89:	 0000                 	addb	%al, (%rax)
;;   8b:	 0000                 	addb	%al, (%rax)
;;   8d:	 0000                 	addb	%al, (%rax)
;;   8f:	 0000                 	addb	%al, (%rax)
;;   91:	 0000                 	addb	%al, (%rax)
;;   93:	 0000                 	addb	%al, (%rax)
;;   95:	 00f0                 	addb	%dh, %al
