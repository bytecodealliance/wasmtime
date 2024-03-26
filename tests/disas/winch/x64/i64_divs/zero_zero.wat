;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0)
	(i64.const 0)
	(i64.div_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8733000000         	ja	0x4e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 48c7c100000000       	movq	$0, %rcx
;;      	 48c7c000000000       	movq	$0, %rax
;;      	 4883f900             	cmpq	$0, %rcx
;;      	 0f840d000000         	je	0x50
;;   43:	 4899                 	cqto	
;;      	 48f7f9               	idivq	%rcx
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4e:	 0f0b                 	ud2	
;;   50:	 0f0b                 	ud2	
