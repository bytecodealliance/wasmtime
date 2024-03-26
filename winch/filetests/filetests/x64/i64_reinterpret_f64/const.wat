;;! target = "x86_64"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.reinterpret_f64)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8723000000         	ja	0x3e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f10050d000000     	movsd	0xd(%rip), %xmm0
;;      	 66480f7ec0           	movq	%xmm0, %rax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   3e:	 0f0b                 	ud2	
;;   40:	 0000                 	addb	%al, (%rax)
;;   42:	 0000                 	addb	%al, (%rax)
;;   44:	 0000                 	addb	%al, (%rax)
