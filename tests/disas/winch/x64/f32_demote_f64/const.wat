;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f64.const 1.0)
        (f32.demote_f64)
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
;;      	 f20f10050d000000     	movsd	0xd(%rip), %xmm0
;;      	 f20f5ac0             	cvtsd2ss	%xmm0, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   3d:	 0f0b                 	ud2	
;;   3f:	 0000                 	addb	%al, (%rax)
;;   41:	 0000                 	addb	%al, (%rax)
;;   43:	 0000                 	addb	%al, (%rax)
;;   45:	 00f0                 	addb	%dh, %al
