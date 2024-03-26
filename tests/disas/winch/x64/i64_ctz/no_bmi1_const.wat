;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (i64.const 1)
        (i64.ctz)
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
;;      	 48c7c001000000       	movq	$1, %rax
;;      	 480fbcc0             	bsfq	%rax, %rax
;;      	 41bb00000000         	movl	$0, %r11d
;;      	 410f94c3             	sete	%r11b
;;      	 49c1e306             	shlq	$6, %r11
;;      	 4c01d8               	addq	%r11, %rax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4d:	 0f0b                 	ud2	
