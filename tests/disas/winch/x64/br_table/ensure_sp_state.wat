;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (result i32)
    block (result i32)
       i32.const 0
    end
    i32.const 0
    i32.const 0
    br_table 0
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c314000000       	addq	$0x14, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f874f000000         	ja	0x6a
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b800000000           	movl	$0, %eax
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 b900000000           	movl	$0, %ecx
;;      	 b800000000           	movl	$0, %eax
;;      	 ba00000000           	movl	$0, %edx
;;      	 39ca                 	cmpl	%ecx, %edx
;;      	 0f42ca               	cmovbl	%edx, %ecx
;;      	 4c8d1d0a000000       	leaq	0xa(%rip), %r11
;;      	 4963148b             	movslq	(%r11, %rcx, 4), %rdx
;;      	 4901d3               	addq	%rdx, %r11
;;      	 41ffe3               	jmpq	*%r11
;;   5c:	 0400                 	addb	$0, %al
;;      	 0000                 	addb	%al, (%rax)
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   6a:	 0f0b                 	ud2	
