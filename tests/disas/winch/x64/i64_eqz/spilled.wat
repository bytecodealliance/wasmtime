;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        i64.const 1
        i64.eqz
        block
        end
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c314000000       	addq	$0x14, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8738000000         	ja	0x53
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 48c7c001000000       	movq	$1, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 b800000000           	movl	$0, %eax
;;      	 400f94c0             	sete	%al
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   53:	 0f0b                 	ud2	
