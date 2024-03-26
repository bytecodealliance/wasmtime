;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (param i64) (result i32)
        (local.get 0)
        (local.get 1)
        (i64.gt_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8737000000         	ja	0x52
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4889542408           	movq	%rdx, 8(%rsp)
;;      	 48890c24             	movq	%rcx, (%rsp)
;;      	 488b0424             	movq	(%rsp), %rax
;;      	 488b4c2408           	movq	8(%rsp), %rcx
;;      	 4839c1               	cmpq	%rax, %rcx
;;      	 b900000000           	movl	$0, %ecx
;;      	 400f97c1             	seta	%cl
;;      	 89c8                 	movl	%ecx, %eax
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   52:	 0f0b                 	ud2	
