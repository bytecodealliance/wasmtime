;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.le
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8751000000         	ja	0x6c
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f30f100534000000     	movss	0x34(%rip), %xmm0
;;      	 f30f11442404         	movss	%xmm0, 4(%rsp)
;;      	 f30f10052e000000     	movss	0x2e(%rip), %xmm0
;;      	 f30f110424           	movss	%xmm0, (%rsp)
;;      	 f30f100424           	movss	(%rsp), %xmm0
;;      	 f30f104c2404         	movss	4(%rsp), %xmm1
;;      	 0f2ec1               	ucomiss	%xmm1, %xmm0
;;      	 b800000000           	movl	$0, %eax
;;      	 400f93c0             	setae	%al
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   6c:	 0f0b                 	ud2	
;;   6e:	 0000                 	addb	%al, (%rax)
;;   70:	 cdcc                 	int	$0xcc
