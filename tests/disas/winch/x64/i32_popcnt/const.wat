;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has-popcnt", "-Ccranelift-has-sse42"]

(module
    (func (result i32)
      i32.const 3
      i32.popcnt
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f871f000000         	ja	0x3a
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b803000000           	movl	$3, %eax
;;      	 f30fb8c0             	popcntl	%eax, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq
;;   3a:	 0f0b                 	ud2
