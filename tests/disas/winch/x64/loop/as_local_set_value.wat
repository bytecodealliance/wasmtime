;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-local-set-value") (result i32)
    (local i32) (local.set 0 (loop (result i32) (i32.const 1))) (local.get 0)
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f872c000000         	ja	0x47
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 b801000000           	movl	$1, %eax
;;      	 89442404             	movl	%eax, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   47:	 0f0b                 	ud2	
