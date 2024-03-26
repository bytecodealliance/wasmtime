;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
        (local.get 0)
        (local.get 1)
        (i32.eq)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8732000000         	ja	0x4d
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 39c1                 	cmpl	%eax, %ecx
;;      	 b900000000           	movl	$0, %ecx
;;      	 400f94c1             	sete	%cl
;;      	 89c8                 	movl	%ecx, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4d:	 0f0b                 	ud2	
