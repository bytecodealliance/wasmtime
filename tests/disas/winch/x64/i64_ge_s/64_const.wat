;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 9223372036854775806)
        (i64.const 9223372036854775807)
        (i64.ge_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8736000000         	ja	0x51
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 48b8feffffffffffff7f 	
;; 				movabsq	$0x7ffffffffffffffe, %rax
;;      	 49bbffffffffffffff7f 	
;; 				movabsq	$0x7fffffffffffffff, %r11
;;      	 4c39d8               	cmpq	%r11, %rax
;;      	 b800000000           	movl	$0, %eax
;;      	 400f9dc0             	setge	%al
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   51:	 0f0b                 	ud2	
