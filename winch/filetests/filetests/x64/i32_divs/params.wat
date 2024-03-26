;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.div_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8731000000         	ja	0x4c
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 83f900               	cmpl	$0, %ecx
;;      	 0f840b000000         	je	0x4e
;;   43:	 99                   	cltd	
;;      	 f7f9                 	idivl	%ecx
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4c:	 0f0b                 	ud2	
;;   4e:	 0f0b                 	ud2	
