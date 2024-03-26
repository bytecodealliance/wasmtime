;;! target = "x86_64"
(module
  (table $t3 3 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)
  (func $f3 (export "get-funcref") (param $i i32) (result funcref)
    (table.get $t3 (local.get $i))
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
;;      	 0f8792000000         	ja	0xad
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c89f2               	movq	%r14, %rdx
;;      	 8b5a50               	movl	0x50(%rdx), %ebx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 0f835d000000         	jae	0xaf
;;   52:	 4189cb               	movl	%ecx, %r11d
;;      	 4d6bdb08             	imulq	$8, %r11, %r11
;;      	 488b5248             	movq	0x48(%rdx), %rdx
;;      	 4889d6               	movq	%rdx, %rsi
;;      	 4c01da               	addq	%r11, %rdx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 480f43d6             	cmovaeq	%rsi, %rdx
;;      	 488b02               	movq	(%rdx), %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f852e000000         	jne	0xa3
;;   75:	 4883ec04             	subq	$4, %rsp
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 4883ec04             	subq	$4, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 8b542404             	movl	4(%rsp), %edx
;;      	 e800000000           	callq	0x91
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 e904000000           	jmp	0xa7
;;   a3:	 4883e0fe             	andq	$0xfffffffffffffffe, %rax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   ad:	 0f0b                 	ud2	
;;   af:	 0f0b                 	ud2	
