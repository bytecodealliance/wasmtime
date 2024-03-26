;;! target = "x86_64"
;;! test = "winch"
(module
  (func $f (param i32 i32 i32) (result i32) (i32.const -1))
  (func (export "as-call-first") (result i32)
    (block (result i32)
      (call $f
        (br_if 0 (i32.const 12) (i32.const 1)) (i32.const 2) (i32.const 3)
      )
    )
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8729000000         	ja	0x44
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 8954240c             	movl	%edx, 0xc(%rsp)
;;      	 894c2408             	movl	%ecx, 8(%rsp)
;;      	 4489442404           	movl	%r8d, 4(%rsp)
;;      	 b8ffffffff           	movl	$0xffffffff, %eax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   44:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f875a000000         	ja	0x75
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b901000000           	movl	$1, %ecx
;;      	 b80c000000           	movl	$0xc, %eax
;;      	 85c9                 	testl	%ecx, %ecx
;;      	 0f8532000000         	jne	0x6f
;;   3d:	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 4883ec0c             	subq	$0xc, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 8b54240c             	movl	0xc(%rsp), %edx
;;      	 b902000000           	movl	$2, %ecx
;;      	 41b803000000         	movl	$3, %r8d
;;      	 e800000000           	callq	0x62
;;      	 4883c40c             	addq	$0xc, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   75:	 0f0b                 	ud2	
