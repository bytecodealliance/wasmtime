;;! target = "x86_64"
(module
  (func $f (param i32 i32 i32) (result i32) (i32.const -1))
  (func (export "as-call-all") (result i32)
    (block (result i32) (call $f (br 0 (i32.const 15))))
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
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f871b000000         	ja	0x36
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b80f000000           	movl	$0xf, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   36:	 0f0b                 	ud2	
