;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 0x80000000)
	(i32.const -1)
	(i32.rem_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8738000000         	ja	0x53
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b9ffffffff           	movl	$0xffffffff, %ecx
;;      	 b800000080           	movl	$0x80000000, %eax
;;      	 99                   	cltd	
;;      	 83f9ff               	cmpl	$-1, %ecx
;;      	 0f850a000000         	jne	0x49
;;   3f:	 ba00000000           	movl	$0, %edx
;;      	 e902000000           	jmp	0x4b
;;   49:	 f7f9                 	idivl	%ecx
;;      	 89d0                 	movl	%edx, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   53:	 0f0b                 	ud2	
