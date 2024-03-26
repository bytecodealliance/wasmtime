;;! target = "x86_64"

(module
    (func (result f32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const -1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.copysign
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8762000000         	ja	0x7d
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f30f100544000000     	movss	0x44(%rip), %xmm0
;;      	 f30f11442404         	movss	%xmm0, 4(%rsp)
;;      	 f30f10053e000000     	movss	0x3e(%rip), %xmm0
;;      	 f30f110424           	movss	%xmm0, (%rsp)
;;      	 f30f100424           	movss	(%rsp), %xmm0
;;      	 f30f104c2404         	movss	4(%rsp), %xmm1
;;      	 41bb00000080         	movl	$0x80000000, %r11d
;;      	 66450f6efb           	movd	%r11d, %xmm15
;;      	 410f54c7             	andps	%xmm15, %xmm0
;;      	 440f55f9             	andnps	%xmm1, %xmm15
;;      	 410f28cf             	movaps	%xmm15, %xmm1
;;      	 0f56c8               	orps	%xmm0, %xmm1
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   7d:	 0f0b                 	ud2	
;;   7f:	 00cd                 	addb	%cl, %ch
;;   81:	 cc                   	int3	
