;;! target = "x86_64"

(module
    (func (result f32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.min
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f876d000000         	ja	0x88
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f30f100554000000     	movss	0x54(%rip), %xmm0
;;      	 f30f11442404         	movss	%xmm0, 4(%rsp)
;;      	 f30f10054e000000     	movss	0x4e(%rip), %xmm0
;;      	 f30f110424           	movss	%xmm0, (%rsp)
;;      	 f30f100424           	movss	(%rsp), %xmm0
;;      	 f30f104c2404         	movss	4(%rsp), %xmm1
;;      	 0f2ec8               	ucomiss	%xmm0, %xmm1
;;      	 0f8518000000         	jne	0x7b
;;      	 0f8a08000000         	jp	0x71
;;   69:	 0f56c8               	orps	%xmm0, %xmm1
;;      	 e90e000000           	jmp	0x7f
;;   71:	 f30f58c8             	addss	%xmm0, %xmm1
;;      	 0f8a04000000         	jp	0x7f
;;   7b:	 f30f5dc8             	minss	%xmm0, %xmm1
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   88:	 0f0b                 	ud2	
;;   8a:	 0000                 	addb	%al, (%rax)
;;   8c:	 0000                 	addb	%al, (%rax)
;;   8e:	 0000                 	addb	%al, (%rax)
;;   90:	 cdcc                 	int	$0xcc
