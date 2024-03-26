;;! target = "x86_64"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.max
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8774000000         	ja	0x8f
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4531db               	xorl	%r11d, %r11d
;;      	 4c895c2408           	movq	%r11, 8(%rsp)
;;      	 4c891c24             	movq	%r11, (%rsp)
;;      	 f20f100558000000     	movsd	0x58(%rip), %xmm0
;;      	 f20f11442408         	movsd	%xmm0, 8(%rsp)
;;      	 f20f100552000000     	movsd	0x52(%rip), %xmm0
;;      	 f20f110424           	movsd	%xmm0, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 f20f104c2408         	movsd	8(%rsp), %xmm1
;;      	 660f2ec8             	ucomisd	%xmm0, %xmm1
;;      	 0f8519000000         	jne	0x81
;;      	 0f8a09000000         	jp	0x77
;;   6e:	 660f54c8             	andpd	%xmm0, %xmm1
;;      	 e90e000000           	jmp	0x85
;;   77:	 f20f58c8             	addsd	%xmm0, %xmm1
;;      	 0f8a04000000         	jp	0x85
;;   81:	 f20f5fc8             	maxsd	%xmm0, %xmm1
;;      	 660f28c1             	movapd	%xmm1, %xmm0
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   8f:	 0f0b                 	ud2	
;;   91:	 0000                 	addb	%al, (%rax)
;;   93:	 0000                 	addb	%al, (%rax)
;;   95:	 0000                 	addb	%al, (%rax)
;;   97:	 009a99999999         	addb	%bl, -0x66666667(%rdx)
;;   9d:	 99                   	cltd	
;;   9e:	 f1                   	int1	
