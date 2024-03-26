;;! target = "x86_64"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.rem_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8745000000         	ja	0x60
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4889542408           	movq	%rdx, 8(%rsp)
;;      	 48890c24             	movq	%rcx, (%rsp)
;;      	 488b0c24             	movq	(%rsp), %rcx
;;      	 488b442408           	movq	8(%rsp), %rax
;;      	 4899                 	cqto	
;;      	 4883f9ff             	cmpq	$-1, %rcx
;;      	 0f850a000000         	jne	0x54
;;   4a:	 ba00000000           	movl	$0, %edx
;;      	 e903000000           	jmp	0x57
;;   54:	 48f7f9               	idivq	%rcx
;;      	 4889d0               	movq	%rdx, %rax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   60:	 0f0b                 	ud2	
