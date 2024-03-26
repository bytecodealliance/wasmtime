;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo i32)
        (local $bar i32)

        (i32.const 2)
        (local.set $foo)
        (i32.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i32.lt_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8744000000         	ja	0x5f
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 b802000000           	movl	$2, %eax
;;      	 89442404             	movl	%eax, 4(%rsp)
;;      	 b803000000           	movl	$3, %eax
;;      	 890424               	movl	%eax, (%rsp)
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 39c1                 	cmpl	%eax, %ecx
;;      	 b900000000           	movl	$0, %ecx
;;      	 400f9cc1             	setl	%cl
;;      	 89c8                 	movl	%ecx, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5f:	 0f0b                 	ud2	
