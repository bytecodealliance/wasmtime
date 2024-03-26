;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (result i64)
        (local.get 0)
        (i64.clz)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8737000000         	ja	0x52
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48891424             	movq	%rdx, (%rsp)
;;      	 488b0424             	movq	(%rsp), %rax
;;      	 480fbdc0             	bsrq	%rax, %rax
;;      	 41bb00000000         	movl	$0, %r11d
;;      	 410f95c3             	setne	%r11b
;;      	 48f7d8               	negq	%rax
;;      	 4883c040             	addq	$0x40, %rax
;;      	 4c29d8               	subq	%r11, %rax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   52:	 0f0b                 	ud2	
