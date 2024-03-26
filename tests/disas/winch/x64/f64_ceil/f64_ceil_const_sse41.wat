;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["has_sse41"]

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.ceil)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8724000000         	ja	0x3f
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f100515000000     	movsd	0x15(%rip), %xmm0
;;      	 660f3a0bc002         	roundsd	$2, %xmm0, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   3f:	 0f0b                 	ud2	
;;   41:	 0000                 	addb	%al, (%rax)
;;   43:	 0000                 	addb	%al, (%rax)
;;   45:	 0000                 	addb	%al, (%rax)
;;   47:	 001f                 	addb	%bl, (%rdi)
;;   49:	 85eb                 	testl	%ebp, %ebx
;;   4b:	 51                   	pushq	%rcx
