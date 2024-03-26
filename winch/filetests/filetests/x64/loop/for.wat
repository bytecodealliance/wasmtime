;;! target = "x86_64"
(module
  (func (export "for-") (param i64) (result i64)
    (local i64 i64)
    (local.set 1 (i64.const 1))
    (local.set 2 (i64.const 2))
    (block
      (loop
        (br_if 1 (i64.gt_u (local.get 2) (local.get 0)))
        (local.set 1 (i64.mul (local.get 1) (local.get 2)))
        (local.set 2 (i64.add (local.get 2) (i64.const 1)))
        (br 0)
      )
    )
    (local.get 1)
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c328000000       	addq	$0x28, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8784000000         	ja	0x9f
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec28             	subq	$0x28, %rsp
;;      	 48897c2420           	movq	%rdi, 0x20(%rsp)
;;      	 4889742418           	movq	%rsi, 0x18(%rsp)
;;      	 4889542410           	movq	%rdx, 0x10(%rsp)
;;      	 4531db               	xorl	%r11d, %r11d
;;      	 4c895c2408           	movq	%r11, 8(%rsp)
;;      	 4c891c24             	movq	%r11, (%rsp)
;;      	 48c7c001000000       	movq	$1, %rax
;;      	 4889442408           	movq	%rax, 8(%rsp)
;;      	 48c7c002000000       	movq	$2, %rax
;;      	 48890424             	movq	%rax, (%rsp)
;;      	 488b442410           	movq	0x10(%rsp), %rax
;;      	 488b0c24             	movq	(%rsp), %rcx
;;      	 4839c1               	cmpq	%rax, %rcx
;;      	 b900000000           	movl	$0, %ecx
;;      	 400f97c1             	seta	%cl
;;      	 85c9                 	testl	%ecx, %ecx
;;      	 0f8523000000         	jne	0x94
;;   71:	 488b0424             	movq	(%rsp), %rax
;;      	 488b4c2408           	movq	8(%rsp), %rcx
;;      	 480fafc8             	imulq	%rax, %rcx
;;      	 48894c2408           	movq	%rcx, 8(%rsp)
;;      	 488b0424             	movq	(%rsp), %rax
;;      	 4883c001             	addq	$1, %rax
;;      	 48890424             	movq	%rax, (%rsp)
;;      	 e9c0ffffff           	jmp	0x54
;;   94:	 488b442408           	movq	8(%rsp), %rax
;;      	 4883c428             	addq	$0x28, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   9f:	 0f0b                 	ud2	
