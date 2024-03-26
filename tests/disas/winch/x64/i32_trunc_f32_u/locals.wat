;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local f32)  

        (local.get 0)
        (i32.trunc_f32_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f876a000000         	ja	0x85
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f30f104c2404         	movss	4(%rsp), %xmm1
;;      	 41bb0000004f         	movl	$0x4f000000, %r11d
;;      	 66450f6efb           	movd	%r11d, %xmm15
;;      	 410f2ecf             	ucomiss	%xmm15, %xmm1
;;      	 0f8315000000         	jae	0x64
;;      	 0f8a32000000         	jp	0x87
;;   55:	 f30f2cc1             	cvttss2si	%xmm1, %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8d1d000000         	jge	0x7f
;;   62:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 f3410f5cc7           	subss	%xmm15, %xmm0
;;      	 f30f2cc0             	cvttss2si	%xmm0, %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8c10000000         	jl	0x89
;;   79:	 81c000000080         	addl	$0x80000000, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   85:	 0f0b                 	ud2	
;;   87:	 0f0b                 	ud2	
;;   89:	 0f0b                 	ud2	
