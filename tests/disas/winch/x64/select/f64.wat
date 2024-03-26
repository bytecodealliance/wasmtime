;;! target = "x86_64"

(module
  (func (export "select-f64") (param f64 f64 i32) (result f64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c328000000       	addq	$0x28, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8744000000         	ja	0x5f
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec28             	subq	$0x28, %rsp
;;      	 48897c2420           	movq	%rdi, 0x20(%rsp)
;;      	 4889742418           	movq	%rsi, 0x18(%rsp)
;;      	 f20f11442410         	movsd	%xmm0, 0x10(%rsp)
;;      	 f20f114c2408         	movsd	%xmm1, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 f20f10442408         	movsd	8(%rsp), %xmm0
;;      	 f20f104c2410         	movsd	0x10(%rsp), %xmm1
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8404000000         	je	0x59
;;   55:	 f20f10c1             	movsd	%xmm1, %xmm0
;;      	 4883c428             	addq	$0x28, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5f:	 0f0b                 	ud2	
