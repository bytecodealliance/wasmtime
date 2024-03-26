;;! target = "x86_64"

(module
    (func (param f32) (param f32) (result i32)
        (local.get 0)
        (local.get 1)
        (f32.eq)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8746000000         	ja	0x61
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 f30f11442404         	movss	%xmm0, 4(%rsp)
;;      	 f30f110c24           	movss	%xmm1, (%rsp)
;;      	 f30f100424           	movss	(%rsp), %xmm0
;;      	 f30f104c2404         	movss	4(%rsp), %xmm1
;;      	 0f2ec8               	ucomiss	%xmm0, %xmm1
;;      	 b800000000           	movl	$0, %eax
;;      	 400f94c0             	sete	%al
;;      	 41bb00000000         	movl	$0, %r11d
;;      	 410f9bc3             	setnp	%r11b
;;      	 4421d8               	andl	%r11d, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   61:	 0f0b                 	ud2	
