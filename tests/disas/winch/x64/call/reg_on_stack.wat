;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "") (param i32) (result i32)
    local.get 0
    i32.const 1
    call 0
    i32.const 1
    call 0
    br_if 0 (;@0;)
    unreachable
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c324000000       	addq	$0x24, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8789000000         	ja	0xa4
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 4883ec04             	subq	$4, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 ba01000000           	movl	$1, %edx
;;      	 e800000000           	callq	0x51
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742414           	movq	0x14(%rsp), %r14
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 ba01000000           	movl	$1, %edx
;;      	 e800000000           	callq	0x71
;;      	 4c8b742418           	movq	0x18(%rsp), %r14
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 4883c404             	addq	$4, %rsp
;;      	 85c9                 	testl	%ecx, %ecx
;;      	 0f8409000000         	je	0x9c
;;   93:	 4883c404             	addq	$4, %rsp
;;      	 e902000000           	jmp	0x9e
;;   9c:	 0f0b                 	ud2	
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   a4:	 0f0b                 	ud2	
