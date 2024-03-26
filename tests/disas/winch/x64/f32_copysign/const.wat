;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const -1.1)
        (f32.const 2.2)
        (f32.copysign)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8743000000         	ja	0x5e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f30f10052d000000     	movss	0x2d(%rip), %xmm0
;;      	 f30f100d2d000000     	movss	0x2d(%rip), %xmm1
;;      	 41bb00000080         	movl	$0x80000000, %r11d
;;      	 66450f6efb           	movd	%r11d, %xmm15
;;      	 410f54c7             	andps	%xmm15, %xmm0
;;      	 440f55f9             	andnps	%xmm1, %xmm15
;;      	 410f28cf             	movaps	%xmm15, %xmm1
;;      	 0f56c8               	orps	%xmm0, %xmm1
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5e:	 0f0b                 	ud2	
;;   60:	 cdcc                 	int	$0xcc
;;   62:	 0c40                 	orb	$0x40, %al
;;   64:	 0000                 	addb	%al, (%rax)
;;   66:	 0000                 	addb	%al, (%rax)
;;   68:	 cdcc                 	int	$0xcc
