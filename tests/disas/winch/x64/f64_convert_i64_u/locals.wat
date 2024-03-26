;;! target = "x86_64"

(module
    (func (result f64)
        (local i64)  

        (local.get 0)
        (f64.convert_i64_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8751000000         	ja	0x6c
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 488b0c24             	movq	(%rsp), %rcx
;;      	 4883f900             	cmpq	$0, %rcx
;;      	 0f8c0a000000         	jl	0x4c
;;   42:	 f2480f2ac1           	cvtsi2sdq	%rcx, %xmm0
;;      	 e91a000000           	jmp	0x66
;;   4c:	 4989cb               	movq	%rcx, %r11
;;      	 49c1eb01             	shrq	$1, %r11
;;      	 4889c8               	movq	%rcx, %rax
;;      	 4883e001             	andq	$1, %rax
;;      	 4c09d8               	orq	%r11, %rax
;;      	 f2480f2ac0           	cvtsi2sdq	%rax, %xmm0
;;      	 f20f58c0             	addsd	%xmm0, %xmm0
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   6c:	 0f0b                 	ud2	
