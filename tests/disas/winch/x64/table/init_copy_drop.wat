;;! target = "x86_64"
;;! test = "winch"

(module
  (type (func (result i32)))  ;; type #0
  (import "a" "ef0" (func (result i32)))    ;; index 0
  (import "a" "ef1" (func (result i32)))
  (import "a" "ef2" (func (result i32)))
  (import "a" "ef3" (func (result i32)))
  (import "a" "ef4" (func (result i32)))    ;; index 4
  (table $t0 30 30 funcref)
  (table $t1 30 30 funcref)
  (elem (table $t0) (i32.const 2) func 3 1 4 1)
  (elem funcref
    (ref.func 2) (ref.func 7) (ref.func 1) (ref.func 8))
  (elem (table $t0) (i32.const 12) func 7 5 2 3 6)
  (elem funcref
    (ref.func 5) (ref.func 9) (ref.func 2) (ref.func 7) (ref.func 6))
  (func (result i32) (i32.const 5))  ;; index 5
  (func (result i32) (i32.const 6))
  (func (result i32) (i32.const 7))
  (func (result i32) (i32.const 8))
  (func (result i32) (i32.const 9))  ;; index 9
  (func (export "test")
    (table.init $t0 1 (i32.const 7) (i32.const 0) (i32.const 4))
         (elem.drop 1)
         (table.init $t0 3 (i32.const 15) (i32.const 1) (i32.const 3))
         (elem.drop 3)
         (table.copy $t0 0 (i32.const 20) (i32.const 15) (i32.const 5))
         (table.copy $t0 0 (i32.const 21) (i32.const 29) (i32.const 1))
         (table.copy $t0 0 (i32.const 24) (i32.const 10) (i32.const 1))
         (table.copy $t0 0 (i32.const 13) (i32.const 11) (i32.const 4))
         (table.copy $t0 0 (i32.const 19) (i32.const 20) (i32.const 5)))
  (func (export "check") (param i32) (result i32)
    (call_indirect $t0 (type 0) (local.get 0)))
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f871b000000         	ja	0x36
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b805000000           	movl	$5, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f871b000000         	ja	0x36
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b806000000           	movl	$6, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f871b000000         	ja	0x36
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b807000000           	movl	$7, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f871b000000         	ja	0x36
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b808000000           	movl	$8, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f871b000000         	ja	0x36
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b809000000           	movl	$9, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8752010000         	ja	0x16d
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 ba01000000           	movl	$1, %edx
;;      	 b907000000           	movl	$7, %ecx
;;      	 41b800000000         	movl	$0, %r8d
;;      	 41b904000000         	movl	$4, %r9d
;;      	 e800000000           	callq	0x4e
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be01000000           	movl	$1, %esi
;;      	 e800000000           	callq	0x60
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 ba03000000           	movl	$3, %edx
;;      	 b90f000000           	movl	$0xf, %ecx
;;      	 41b801000000         	movl	$1, %r8d
;;      	 41b903000000         	movl	$3, %r9d
;;      	 e800000000           	callq	0x88
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be03000000           	movl	$3, %esi
;;      	 e800000000           	callq	0x9a
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 ba00000000           	movl	$0, %edx
;;      	 b914000000           	movl	$0x14, %ecx
;;      	 41b80f000000         	movl	$0xf, %r8d
;;      	 41b905000000         	movl	$5, %r9d
;;      	 e800000000           	callq	0xc2
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 ba00000000           	movl	$0, %edx
;;      	 b915000000           	movl	$0x15, %ecx
;;      	 41b81d000000         	movl	$0x1d, %r8d
;;      	 41b901000000         	movl	$1, %r9d
;;      	 e800000000           	callq	0xea
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 ba00000000           	movl	$0, %edx
;;      	 b918000000           	movl	$0x18, %ecx
;;      	 41b80a000000         	movl	$0xa, %r8d
;;      	 41b901000000         	movl	$1, %r9d
;;      	 e800000000           	callq	0x112
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 ba00000000           	movl	$0, %edx
;;      	 b90d000000           	movl	$0xd, %ecx
;;      	 41b80b000000         	movl	$0xb, %r8d
;;      	 41b904000000         	movl	$4, %r9d
;;      	 e800000000           	callq	0x13a
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 ba00000000           	movl	$0, %edx
;;      	 b913000000           	movl	$0x13, %ecx
;;      	 41b814000000         	movl	$0x14, %r8d
;;      	 41b905000000         	movl	$5, %r9d
;;      	 e800000000           	callq	0x162
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;  16d:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f87d2000000         	ja	0xed
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c89f2               	movq	%r14, %rdx
;;      	 8b9af0000000         	movl	0xf0(%rdx), %ebx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 0f839a000000         	jae	0xef
;;   55:	 4189cb               	movl	%ecx, %r11d
;;      	 4d6bdb08             	imulq	$8, %r11, %r11
;;      	 488b92e8000000       	movq	0xe8(%rdx), %rdx
;;      	 4889d6               	movq	%rdx, %rsi
;;      	 4c01da               	addq	%r11, %rdx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 480f43d6             	cmovaeq	%rsi, %rdx
;;      	 488b02               	movq	(%rdx), %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f852e000000         	jne	0xa9
;;   7b:	 4883ec04             	subq	$4, %rsp
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 4883ec04             	subq	$4, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 8b542404             	movl	4(%rsp), %edx
;;      	 e800000000           	callq	0x97
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 e904000000           	jmp	0xad
;;   a9:	 4883e0fe             	andq	$0xfffffffffffffffe, %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f843b000000         	je	0xf1
;;   b6:	 4d8b5e40             	movq	0x40(%r14), %r11
;;      	 418b0b               	movl	(%r11), %ecx
;;      	 8b5018               	movl	0x18(%rax), %edx
;;      	 39d1                 	cmpl	%edx, %ecx
;;      	 0f852b000000         	jne	0xf3
;;   c8:	 50                   	pushq	%rax
;;      	 59                   	popq	%rcx
;;      	 488b5920             	movq	0x20(%rcx), %rbx
;;      	 488b5110             	movq	0x10(%rcx), %rdx
;;      	 4883ec08             	subq	$8, %rsp
;;      	 4889df               	movq	%rbx, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 ffd2                 	callq	*%rdx
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   ed:	 0f0b                 	ud2	
;;   ef:	 0f0b                 	ud2	
;;   f1:	 0f0b                 	ud2	
;;   f3:	 0f0b                 	ud2	
