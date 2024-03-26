;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i32) (result f32)
        (local.get 0)
        (f32.convert_i32_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f874f000000         	ja	0x6a
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 8bc9                 	movl	%ecx, %ecx
;;      	 4883f900             	cmpq	$0, %rcx
;;      	 0f8c0a000000         	jl	0x4a
;;   40:	 f3480f2ac1           	cvtsi2ssq	%rcx, %xmm0
;;      	 e91a000000           	jmp	0x64
;;   4a:	 4989cb               	movq	%rcx, %r11
;;      	 49c1eb01             	shrq	$1, %r11
;;      	 4889c8               	movq	%rcx, %rax
;;      	 4883e001             	andq	$1, %rax
;;      	 4c09d8               	orq	%r11, %rax
;;      	 f3480f2ac0           	cvtsi2ssq	%rax, %xmm0
;;      	 f30f58c0             	addss	%xmm0, %xmm0
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   6a:	 0f0b                 	ud2	
