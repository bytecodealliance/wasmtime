;;! target = "x86_64"
(module
  (func (export "while-") (param i64) (result i64)
    (local i64)
    (local.set 1 (i64.const 1))
    (block
      (loop
        (br_if 1 (i64.eqz (local.get 0)))
        (local.set 1 (i64.mul (local.get 0) (local.get 1)))
        (local.set 0 (i64.sub (local.get 0) (i64.const 1)))
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
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8771000000         	ja	0x8c
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4889542408           	movq	%rdx, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 48c7c001000000       	movq	$1, %rax
;;      	 48890424             	movq	%rax, (%rsp)
;;      	 488b442408           	movq	8(%rsp), %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 b800000000           	movl	$0, %eax
;;      	 400f94c0             	sete	%al
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8524000000         	jne	0x82
;;   5e:	 488b0424             	movq	(%rsp), %rax
;;      	 488b4c2408           	movq	8(%rsp), %rcx
;;      	 480fafc8             	imulq	%rax, %rcx
;;      	 48890c24             	movq	%rcx, (%rsp)
;;      	 488b442408           	movq	8(%rsp), %rax
;;      	 4883e801             	subq	$1, %rax
;;      	 4889442408           	movq	%rax, 8(%rsp)
;;      	 e9c2ffffff           	jmp	0x44
;;   82:	 488b0424             	movq	(%rsp), %rax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   8c:	 0f0b                 	ud2	
