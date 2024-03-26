;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["has_popcnt"]

(module
    (func (result i32)
      i32.const 3
      i32.popcnt
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8751000000         	ja	0x6c
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b803000000           	movl	$3, %eax
;;      	 89c1                 	movl	%eax, %ecx
;;      	 c1e801               	shrl	$1, %eax
;;      	 81e055555555         	andl	$0x55555555, %eax
;;      	 29c1                 	subl	%eax, %ecx
;;      	 89c8                 	movl	%ecx, %eax
;;      	 41bb33333333         	movl	$0x33333333, %r11d
;;      	 4421d8               	andl	%r11d, %eax
;;      	 c1e902               	shrl	$2, %ecx
;;      	 4421d9               	andl	%r11d, %ecx
;;      	 01c1                 	addl	%eax, %ecx
;;      	 89c8                 	movl	%ecx, %eax
;;      	 c1e804               	shrl	$4, %eax
;;      	 01c8                 	addl	%ecx, %eax
;;      	 81e00f0f0f0f         	andl	$0xf0f0f0f, %eax
;;      	 69c001010101         	imull	$0x1010101, %eax, %eax
;;      	 c1e818               	shrl	$0x18, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   6c:	 0f0b                 	ud2	
