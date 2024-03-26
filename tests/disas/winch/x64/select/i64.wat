;;! target = "x86_64"

(module
  (func (export "select-i64") (param i64 i64 i32) (result i64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c328000000       	addq	$0x28, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f873e000000         	ja	0x59
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec28             	subq	$0x28, %rsp
;;      	 48897c2420           	movq	%rdi, 0x20(%rsp)
;;      	 4889742418           	movq	%rsi, 0x18(%rsp)
;;      	 4889542410           	movq	%rdx, 0x10(%rsp)
;;      	 48894c2408           	movq	%rcx, 8(%rsp)
;;      	 4489442404           	movl	%r8d, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 488b4c2408           	movq	8(%rsp), %rcx
;;      	 488b542410           	movq	0x10(%rsp), %rdx
;;      	 83f800               	cmpl	$0, %eax
;;      	 480f45ca             	cmovneq	%rdx, %rcx
;;      	 4889c8               	movq	%rcx, %rax
;;      	 4883c428             	addq	$0x28, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   59:	 0f0b                 	ud2	
