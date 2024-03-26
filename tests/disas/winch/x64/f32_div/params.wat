;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.div)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8734000000         	ja	0x4f
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 f30f11442404         	movss	%xmm0, 4(%rsp)
;;      	 f30f110c24           	movss	%xmm1, (%rsp)
;;      	 f30f100424           	movss	(%rsp), %xmm0
;;      	 f30f104c2404         	movss	4(%rsp), %xmm1
;;      	 f30f5ec8             	divss	%xmm0, %xmm1
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4f:	 0f0b                 	ud2	
