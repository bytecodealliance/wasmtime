;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i64)
        (local $bar i64)

        (i64.const 2)
        (local.set $foo)
        (i64.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i64.gt_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8751000000         	ja	0x6c
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4531db               	xorl	%r11d, %r11d
;;      	 4c895c2408           	movq	%r11, 8(%rsp)
;;      	 4c891c24             	movq	%r11, (%rsp)
;;      	 48c7c002000000       	movq	$2, %rax
;;      	 4889442408           	movq	%rax, 8(%rsp)
;;      	 48c7c003000000       	movq	$3, %rax
;;      	 48890424             	movq	%rax, (%rsp)
;;      	 488b0424             	movq	(%rsp), %rax
;;      	 488b4c2408           	movq	8(%rsp), %rcx
;;      	 4839c1               	cmpq	%rax, %rcx
;;      	 b900000000           	movl	$0, %ecx
;;      	 400f9fc1             	setg	%cl
;;      	 89c8                 	movl	%ecx, %eax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   6c:	 0f0b                 	ud2	
