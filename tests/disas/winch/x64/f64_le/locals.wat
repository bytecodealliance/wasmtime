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
        f64.le
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8756000000         	ja	0x71
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4531db               	xorl	%r11d, %r11d
;;      	 4c895c2408           	movq	%r11, 8(%rsp)
;;      	 4c891c24             	movq	%r11, (%rsp)
;;      	 f20f100538000000     	movsd	0x38(%rip), %xmm0
;;      	 f20f11442408         	movsd	%xmm0, 8(%rsp)
;;      	 f20f100532000000     	movsd	0x32(%rip), %xmm0
;;      	 f20f110424           	movsd	%xmm0, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 f20f104c2408         	movsd	8(%rsp), %xmm1
;;      	 660f2ec1             	ucomisd	%xmm1, %xmm0
;;      	 b800000000           	movl	$0, %eax
;;      	 400f93c0             	setae	%al
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   71:	 0f0b                 	ud2	
;;   73:	 0000                 	addb	%al, (%rax)
;;   75:	 0000                 	addb	%al, (%rax)
;;   77:	 009a99999999         	addb	%bl, -0x66666667(%rdx)
;;   7d:	 99                   	cltd	
;;   7e:	 f1                   	int1	
