;;! target = "x86_64"

(module
    (func (result f64)
        (f32.const 1.0)
        (f64.promote_f32)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8722000000         	ja	0x3d
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f30f10050d000000     	movss	0xd(%rip), %xmm0
;;      	 f30f5ac0             	cvtss2sd	%xmm0, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   3d:	 0f0b                 	ud2	
;;   3f:	 0000                 	addb	%al, (%rax)
