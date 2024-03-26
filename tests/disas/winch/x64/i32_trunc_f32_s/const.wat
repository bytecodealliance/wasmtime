;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8758000000         	ja	0x73
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f30f10054d000000     	movss	0x4d(%rip), %xmm0
;;      	 f30f2cc0             	cvttss2si	%xmm0, %eax
;;      	 83f801               	cmpl	$1, %eax
;;      	 0f812d000000         	jno	0x6d
;;   40:	 0f2ec0               	ucomiss	%xmm0, %xmm0
;;      	 0f8a2c000000         	jp	0x75
;;   49:	 41bb000000cf         	movl	$0xcf000000, %r11d
;;      	 66450f6efb           	movd	%r11d, %xmm15
;;      	 410f2ec7             	ucomiss	%xmm15, %xmm0
;;      	 0f8219000000         	jb	0x77
;;   5e:	 66450f57ff           	xorpd	%xmm15, %xmm15
;;      	 440f2ef8             	ucomiss	%xmm0, %xmm15
;;      	 0f820c000000         	jb	0x79
;;   6d:	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   73:	 0f0b                 	ud2	
;;   75:	 0f0b                 	ud2	
;;   77:	 0f0b                 	ud2	
;;   79:	 0f0b                 	ud2	
;;   7b:	 0000                 	addb	%al, (%rax)
;;   7d:	 0000                 	addb	%al, (%rax)
;;   7f:	 0000                 	addb	%al, (%rax)
