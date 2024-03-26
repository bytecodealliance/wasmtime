;;! target = "x86_64"

(module
    (func (param i64) (result f32)
        (local.get 0)
        (f32.convert_i64_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f874d000000         	ja	0x68
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48891424             	movq	%rdx, (%rsp)
;;      	 488b0c24             	movq	(%rsp), %rcx
;;      	 4883f900             	cmpq	$0, %rcx
;;      	 0f8c0a000000         	jl	0x48
;;   3e:	 f3480f2ac1           	cvtsi2ssq	%rcx, %xmm0
;;      	 e91a000000           	jmp	0x62
;;   48:	 4989cb               	movq	%rcx, %r11
;;      	 49c1eb01             	shrq	$1, %r11
;;      	 4889c8               	movq	%rcx, %rax
;;      	 4883e001             	andq	$1, %rax
;;      	 4c09d8               	orq	%r11, %rax
;;      	 f3480f2ac0           	cvtsi2ssq	%rax, %xmm0
;;      	 f30f58c0             	addss	%xmm0, %xmm0
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   68:	 0f0b                 	ud2	
