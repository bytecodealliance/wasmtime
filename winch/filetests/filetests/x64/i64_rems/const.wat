;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 7)
	(i64.const 5)
	(i64.rem_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8740000000         	ja	0x5b
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 48c7c105000000       	movq	$5, %rcx
;;      	 48c7c007000000       	movq	$7, %rax
;;      	 4899                 	cqto	
;;      	 4883f9ff             	cmpq	$-1, %rcx
;;      	 0f850a000000         	jne	0x4f
;;   45:	 ba00000000           	movl	$0, %edx
;;      	 e903000000           	jmp	0x52
;;   4f:	 48f7f9               	idivq	%rcx
;;      	 4889d0               	movq	%rdx, %rax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5b:	 0f0b                 	ud2	
