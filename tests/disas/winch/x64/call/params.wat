;;! target = "x86_64"

(module
  (func (export "main") (param i32) (param i32) (result i32)
    (local.get 1)
    (local.get 0)
    (i32.div_u)

    (call $add (i32.const 1) (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))

    (local.get 1)
    (local.get 0)
    (i32.div_u)

    (call $add (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))
  )

  (func $add (param i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.add)
    (local.get 2)
    (i32.add)
    (local.get 3)
    (i32.add)
    (local.get 4)
    (i32.add)
    (local.get 5)
    (i32.add)
    (local.get 6)
    (i32.add)
    (local.get 7)
    (i32.add)
    (local.get 8)
    (i32.add)
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c350000000       	addq	$0x50, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8737010000         	ja	0x152
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 31d2                 	xorl	%edx, %edx
;;      	 f7f1                 	divl	%ecx
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 4883ec34             	subq	$0x34, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 8b542434             	movl	0x34(%rsp), %edx
;;      	 b901000000           	movl	$1, %ecx
;;      	 41b802000000         	movl	$2, %r8d
;;      	 41b903000000         	movl	$3, %r9d
;;      	 41bb04000000         	movl	$4, %r11d
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 41bb05000000         	movl	$5, %r11d
;;      	 44895c2408           	movl	%r11d, 8(%rsp)
;;      	 41bb06000000         	movl	$6, %r11d
;;      	 44895c2410           	movl	%r11d, 0x10(%rsp)
;;      	 41bb07000000         	movl	$7, %r11d
;;      	 44895c2418           	movl	%r11d, 0x18(%rsp)
;;      	 41bb08000000         	movl	$8, %r11d
;;      	 44895c2420           	movl	%r11d, 0x20(%rsp)
;;      	 e800000000           	callq	0x9f
;;      	 4883c434             	addq	$0x34, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 448b5c240c           	movl	0xc(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 4883c404             	addq	$4, %rsp
;;      	 31d2                 	xorl	%edx, %edx
;;      	 f7f1                 	divl	%ecx
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 4883ec30             	subq	$0x30, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 8b542434             	movl	0x34(%rsp), %edx
;;      	 8b4c2430             	movl	0x30(%rsp), %ecx
;;      	 41b802000000         	movl	$2, %r8d
;;      	 41b903000000         	movl	$3, %r9d
;;      	 41bb04000000         	movl	$4, %r11d
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 41bb05000000         	movl	$5, %r11d
;;      	 44895c2408           	movl	%r11d, 8(%rsp)
;;      	 41bb06000000         	movl	$6, %r11d
;;      	 44895c2410           	movl	%r11d, 0x10(%rsp)
;;      	 41bb07000000         	movl	$7, %r11d
;;      	 44895c2418           	movl	%r11d, 0x18(%rsp)
;;      	 41bb08000000         	movl	$8, %r11d
;;      	 44895c2420           	movl	%r11d, 0x20(%rsp)
;;      	 e800000000           	callq	0x13f
;;      	 4883c430             	addq	$0x30, %rsp
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;  152:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8758000000         	ja	0x73
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 8954240c             	movl	%edx, 0xc(%rsp)
;;      	 894c2408             	movl	%ecx, 8(%rsp)
;;      	 4489442404           	movl	%r8d, 4(%rsp)
;;      	 44890c24             	movl	%r9d, (%rsp)
;;      	 8b442408             	movl	8(%rsp), %eax
;;      	 8b4c240c             	movl	0xc(%rsp), %ecx
;;      	 01c1                 	addl	%eax, %ecx
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 01c1                 	addl	%eax, %ecx
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 01c1                 	addl	%eax, %ecx
;;      	 8b4510               	movl	0x10(%rbp), %eax
;;      	 01c1                 	addl	%eax, %ecx
;;      	 8b4518               	movl	0x18(%rbp), %eax
;;      	 01c1                 	addl	%eax, %ecx
;;      	 8b4520               	movl	0x20(%rbp), %eax
;;      	 01c1                 	addl	%eax, %ecx
;;      	 8b4528               	movl	0x28(%rbp), %eax
;;      	 01c1                 	addl	%eax, %ecx
;;      	 8b4530               	movl	0x30(%rbp), %eax
;;      	 01c1                 	addl	%eax, %ecx
;;      	 89c8                 	movl	%ecx, %eax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   73:	 0f0b                 	ud2	
