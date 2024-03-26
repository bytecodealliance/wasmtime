;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.add)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f872d000000         	ja	0x48
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f30f10051d000000     	movss	0x1d(%rip), %xmm0
;;      	 f30f100d1d000000     	movss	0x1d(%rip), %xmm1
;;      	 f30f58c8             	addss	%xmm0, %xmm1
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   48:	 0f0b                 	ud2	
;;   4a:	 0000                 	addb	%al, (%rax)
;;   4c:	 0000                 	addb	%al, (%rax)
;;   4e:	 0000                 	addb	%al, (%rax)
;;   50:	 cdcc                 	int	$0xcc
;;   52:	 0c40                 	orb	$0x40, %al
;;   54:	 0000                 	addb	%al, (%rax)
;;   56:	 0000                 	addb	%al, (%rax)
;;   58:	 cdcc                 	int	$0xcc
