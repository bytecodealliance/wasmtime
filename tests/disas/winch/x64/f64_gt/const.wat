;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.gt)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8740000000         	ja	0x5b
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f10052d000000     	movsd	0x2d(%rip), %xmm0
;;      	 f20f100d2d000000     	movsd	0x2d(%rip), %xmm1
;;      	 660f2ec8             	ucomisd	%xmm0, %xmm1
;;      	 b800000000           	movl	$0, %eax
;;      	 400f97c0             	seta	%al
;;      	 41bb00000000         	movl	$0, %r11d
;;      	 410f9bc3             	setnp	%r11b
;;      	 4c21d8               	andq	%r11, %rax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5b:	 0f0b                 	ud2	
;;   5d:	 0000                 	addb	%al, (%rax)
;;   5f:	 009a99999999         	addb	%bl, -0x66666667(%rdx)
;;   65:	 99                   	cltd	
;;   66:	 01409a               	addl	%eax, -0x66(%rax)
;;   69:	 99                   	cltd	
;;   6a:	 99                   	cltd	
;;   6b:	 99                   	cltd	
;;   6c:	 99                   	cltd	
;;   6d:	 99                   	cltd	
;;   6e:	 f1                   	int1	
