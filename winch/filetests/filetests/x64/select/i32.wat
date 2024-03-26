;;! target = "x86_64"

(module
  (func (export "select-i32") (param i32 i32 i32) (result i32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8738000000         	ja	0x53
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 8954240c             	movl	%edx, 0xc(%rsp)
;;      	 894c2408             	movl	%ecx, 8(%rsp)
;;      	 4489442404           	movl	%r8d, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 8b4c2408             	movl	8(%rsp), %ecx
;;      	 8b54240c             	movl	0xc(%rsp), %edx
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f45ca               	cmovnel	%edx, %ecx
;;      	 89c8                 	movl	%ecx, %eax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   53:	 0f0b                 	ud2	
