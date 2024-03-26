;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.ge
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8763000000         	ja	0x7e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4531db               	xorl	%r11d, %r11d
;;      	 4c895c2408           	movq	%r11, 8(%rsp)
;;      	 4c891c24             	movq	%r11, (%rsp)
;;      	 f20f100540000000     	movsd	0x40(%rip), %xmm0
;;      	 f20f11442408         	movsd	%xmm0, 8(%rsp)
;;      	 f20f10053a000000     	movsd	0x3a(%rip), %xmm0
;;      	 f20f110424           	movsd	%xmm0, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 f20f104c2408         	movsd	8(%rsp), %xmm1
;;      	 660f2ec8             	ucomisd	%xmm0, %xmm1
;;      	 b800000000           	movl	$0, %eax
;;      	 400f93c0             	setae	%al
;;      	 41bb00000000         	movl	$0, %r11d
;;      	 410f9bc3             	setnp	%r11b
;;      	 4c21d8               	andq	%r11, %rax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   7e:	 0f0b                 	ud2	
