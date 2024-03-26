;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-local-set-value") (param i32) (result i32)
    (local i32)
    (block (result i32)
      (local.set 0 (br_if 0 (i32.const 17) (local.get 0)))
      (i32.const -1)
    )
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f873f000000         	ja	0x5a
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 c7042400000000       	movl	$0, (%rsp)
;;      	 4531db               	xorl	%r11d, %r11d
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 b811000000           	movl	$0x11, %eax
;;      	 85c9                 	testl	%ecx, %ecx
;;      	 0f8509000000         	jne	0x54
;;   4b:	 89442404             	movl	%eax, 4(%rsp)
;;      	 b8ffffffff           	movl	$0xffffffff, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5a:	 0f0b                 	ud2	
