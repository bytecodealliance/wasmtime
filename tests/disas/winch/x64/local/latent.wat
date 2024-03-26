;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (param i32) (result i32)
    local.get 0
    i32.const 1
    local.set 0
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c31c000000       	addq	$0x1c, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8738000000         	ja	0x53
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 b801000000           	movl	$1, %eax
;;      	 89442408             	movl	%eax, 8(%rsp)
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   53:	 0f0b                 	ud2	
