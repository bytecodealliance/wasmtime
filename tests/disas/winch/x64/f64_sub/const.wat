;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.sub)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f872e000000         	ja	0x49
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f10051d000000     	movsd	0x1d(%rip), %xmm0
;;      	 f20f100d1d000000     	movsd	0x1d(%rip), %xmm1
;;      	 f20f5cc8             	subsd	%xmm0, %xmm1
;;      	 660f28c1             	movapd	%xmm1, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   49:	 0f0b                 	ud2	
;;   4b:	 0000                 	addb	%al, (%rax)
;;   4d:	 0000                 	addb	%al, (%rax)
;;   4f:	 009a99999999         	addb	%bl, -0x66666667(%rdx)
;;   55:	 99                   	cltd	
;;   56:	 01409a               	addl	%eax, -0x66(%rax)
;;   59:	 99                   	cltd	
;;   5a:	 99                   	cltd	
;;   5b:	 99                   	cltd	
;;   5c:	 99                   	cltd	
;;   5d:	 99                   	cltd	
;;   5e:	 f1                   	int1	
