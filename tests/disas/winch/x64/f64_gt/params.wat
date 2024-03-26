;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (param f64) (result i32)
        (local.get 0)
        (local.get 1)
        (f64.gt)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8747000000         	ja	0x62
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 f20f11442408         	movsd	%xmm0, 8(%rsp)
;;      	 f20f110c24           	movsd	%xmm1, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 f20f104c2408         	movsd	8(%rsp), %xmm1
;;      	 660f2ec8             	ucomisd	%xmm0, %xmm1
;;      	 b800000000           	movl	$0, %eax
;;      	 400f97c0             	seta	%al
;;      	 41bb00000000         	movl	$0, %r11d
;;      	 410f9bc3             	setnp	%r11b
;;      	 4c21d8               	andq	%r11, %rax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   62:	 0f0b                 	ud2	
