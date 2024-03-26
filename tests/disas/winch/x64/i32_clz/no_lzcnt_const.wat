;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i32.const 1)
        (i32.clz)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8730000000         	ja	0x4b
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b801000000           	movl	$1, %eax
;;      	 0fbdc0               	bsrl	%eax, %eax
;;      	 41bb00000000         	movl	$0, %r11d
;;      	 410f95c3             	setne	%r11b
;;      	 f7d8                 	negl	%eax
;;      	 83c020               	addl	$0x20, %eax
;;      	 4429d8               	subl	%r11d, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4b:	 0f0b                 	ud2	
