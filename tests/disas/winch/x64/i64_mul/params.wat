;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.mul)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8730000000         	ja	0x4b
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4889542408           	movq	%rdx, 8(%rsp)
;;      	 48890c24             	movq	%rcx, (%rsp)
;;      	 488b0424             	movq	(%rsp), %rax
;;      	 488b4c2408           	movq	8(%rsp), %rcx
;;      	 480fafc8             	imulq	%rax, %rcx
;;      	 4889c8               	movq	%rcx, %rax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4b:	 0f0b                 	ud2	
