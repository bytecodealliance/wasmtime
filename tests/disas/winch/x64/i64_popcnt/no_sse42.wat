;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_popcnt"]

(module
    (func (result i64)
      i64.const 3
      i64.popcnt
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8777000000         	ja	0x92
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 48c7c003000000       	movq	$3, %rax
;;      	 4889c1               	movq	%rax, %rcx
;;      	 48c1e801             	shrq	$1, %rax
;;      	 49bb5555555555555555 	
;; 				movabsq	$0x5555555555555555, %r11
;;      	 4c21d8               	andq	%r11, %rax
;;      	 4829c1               	subq	%rax, %rcx
;;      	 4889c8               	movq	%rcx, %rax
;;      	 49bb3333333333333333 	
;; 				movabsq	$0x3333333333333333, %r11
;;      	 4c21d8               	andq	%r11, %rax
;;      	 48c1e902             	shrq	$2, %rcx
;;      	 4c21d9               	andq	%r11, %rcx
;;      	 4801c1               	addq	%rax, %rcx
;;      	 4889c8               	movq	%rcx, %rax
;;      	 48c1e804             	shrq	$4, %rax
;;      	 4801c8               	addq	%rcx, %rax
;;      	 49bb0f0f0f0f0f0f0f0f 	
;; 				movabsq	$0xf0f0f0f0f0f0f0f, %r11
;;      	 4c21d8               	andq	%r11, %rax
;;      	 49bb0101010101010101 	
;; 				movabsq	$0x101010101010101, %r11
;;      	 490fafc3             	imulq	%r11, %rax
;;      	 48c1e838             	shrq	$0x38, %rax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   92:	 0f0b                 	ud2	
