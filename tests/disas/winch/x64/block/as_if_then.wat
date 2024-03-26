;;! target = "x86_64"
;;! test = "winch"
 (module
   (func (export "as-if-then") (result i32)
      (if (result i32) (i32.const 1) (then (block (result i32) (i32.const 1))) (else (i32.const 2)))
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8732000000         	ja	0x4d
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b801000000           	movl	$1, %eax
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f840a000000         	je	0x42
;;   38:	 b801000000           	movl	$1, %eax
;;      	 e905000000           	jmp	0x47
;;   42:	 b802000000           	movl	$2, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4d:	 0f0b                 	ud2	
