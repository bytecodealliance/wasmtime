;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.max)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8751000000         	ja	0x6c
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f10053d000000     	movsd	0x3d(%rip), %xmm0
;;      	 f20f100d3d000000     	movsd	0x3d(%rip), %xmm1
;;      	 660f2ec8             	ucomisd	%xmm0, %xmm1
;;      	 0f8519000000         	jne	0x5e
;;      	 0f8a09000000         	jp	0x54
;;   4b:	 660f54c8             	andpd	%xmm0, %xmm1
;;      	 e90e000000           	jmp	0x62
;;   54:	 f20f58c8             	addsd	%xmm0, %xmm1
;;      	 0f8a04000000         	jp	0x62
;;   5e:	 f20f5fc8             	maxsd	%xmm0, %xmm1
;;      	 660f28c1             	movapd	%xmm1, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   6c:	 0f0b                 	ud2	
;;   6e:	 0000                 	addb	%al, (%rax)
