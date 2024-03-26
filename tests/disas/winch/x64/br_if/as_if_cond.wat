;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-if-cond") (param i32) (result i32)
    (block (result i32)
      (if (result i32)
        (br_if 0 (i32.const 1) (local.get 0))
        (then (i32.const 2))
        (else (i32.const 3))
      )
    )
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8743000000         	ja	0x5e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 b801000000           	movl	$1, %eax
;;      	 85c9                 	testl	%ecx, %ecx
;;      	 0f8517000000         	jne	0x58
;;   41:	 85c0                 	testl	%eax, %eax
;;      	 0f840a000000         	je	0x53
;;   49:	 b802000000           	movl	$2, %eax
;;      	 e905000000           	jmp	0x58
;;   53:	 b803000000           	movl	$3, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5e:	 0f0b                 	ud2	
