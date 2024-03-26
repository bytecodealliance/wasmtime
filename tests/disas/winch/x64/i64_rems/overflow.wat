;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.rem_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8743000000         	ja	0x5e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 48c7c1ffffffff       	movq	$18446744073709551615, %rcx
;;      	 48b80000000000000080 	
;; 				movabsq	$9223372036854775808, %rax
;;      	 4899                 	cqto	
;;      	 4883f9ff             	cmpq	$-1, %rcx
;;      	 0f850a000000         	jne	0x52
;;   48:	 ba00000000           	movl	$0, %edx
;;      	 e903000000           	jmp	0x55
;;   52:	 48f7f9               	idivq	%rcx
;;      	 4889d0               	movq	%rdx, %rax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5e:	 0f0b                 	ud2	
