;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 0x80000000)
	(i32.const -1)
	(i32.div_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f872c000000         	ja	0x47
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b9ffffffff           	movl	$0xffffffff, %ecx
;;      	 b800000080           	movl	$0x80000000, %eax
;;      	 83f900               	cmpl	$0, %ecx
;;      	 0f840b000000         	je	0x49
;;   3e:	 99                   	cltd	
;;      	 f7f9                 	idivl	%ecx
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   47:	 0f0b                 	ud2	
;;   49:	 0f0b                 	ud2	
