;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-block-last-value") (param i32) (result i32)
    (block (result i32)
      (call $dummy) (call $dummy) (br_if 0 (i32.const 11) (local.get 0))
    )
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8716000000         	ja	0x31
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f875c000000         	ja	0x77
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0x3f
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0x57
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 b80b000000           	movl	$0xb, %eax
;;      	 85c9                 	testl	%ecx, %ecx
;;      	 0f8500000000         	jne	0x71
;;   71:	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   77:	 0f0b                 	ud2	
