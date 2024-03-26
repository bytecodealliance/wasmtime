;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        i64.const 1
        f64.convert_i64_u
        block
        end
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f875d000000         	ja	0x78
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 48c7c101000000       	movq	$1, %rcx
;;      	 4883f900             	cmpq	$0, %rcx
;;      	 0f8c0a000000         	jl	0x46
;;   3c:	 f2480f2ac1           	cvtsi2sdq	%rcx, %xmm0
;;      	 e91a000000           	jmp	0x60
;;   46:	 4989cb               	movq	%rcx, %r11
;;      	 49c1eb01             	shrq	$1, %r11
;;      	 4889c8               	movq	%rcx, %rax
;;      	 4883e001             	andq	$1, %rax
;;      	 4c09d8               	orq	%r11, %rax
;;      	 f2480f2ac0           	cvtsi2sdq	%rax, %xmm0
;;      	 f20f58c0             	addsd	%xmm0, %xmm0
;;      	 4883ec08             	subq	$8, %rsp
;;      	 f20f110424           	movsd	%xmm0, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 4883c408             	addq	$8, %rsp
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   78:	 0f0b                 	ud2	
