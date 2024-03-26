;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.min)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f874e000000         	ja	0x69
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f30f10053d000000     	movss	0x3d(%rip), %xmm0
;;      	 f30f100d3d000000     	movss	0x3d(%rip), %xmm1
;;      	 0f2ec8               	ucomiss	%xmm0, %xmm1
;;      	 0f8518000000         	jne	0x5c
;;      	 0f8a08000000         	jp	0x52
;;   4a:	 0f56c8               	orps	%xmm0, %xmm1
;;      	 e90e000000           	jmp	0x60
;;   52:	 f30f58c8             	addss	%xmm0, %xmm1
;;      	 0f8a04000000         	jp	0x60
;;   5c:	 f30f5dc8             	minss	%xmm0, %xmm1
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   69:	 0f0b                 	ud2	
;;   6b:	 0000                 	addb	%al, (%rax)
;;   6d:	 0000                 	addb	%al, (%rax)
;;   6f:	 00cd                 	addb	%cl, %ch
;;   71:	 cc                   	int3	
;;   72:	 0c40                 	orb	$0x40, %al
;;   74:	 0000                 	addb	%al, (%rax)
;;   76:	 0000                 	addb	%al, (%rax)
;;   78:	 cdcc                 	int	$0xcc
