;;! target = "x86_64"
;;! test = "winch"
(module
  (type $t0 (func))
  (func $f1 (type $t0))
  (func $f2 (type $t0))
  (func $f3 (type $t0))

  ;; Define two tables of funcref
  (table $t1 3 funcref)
  (table $t2 10 funcref)

  ;; Initialize table $t1 with functions $f1, $f2, $f3
  (elem (i32.const 0) $f1 $f2 $f3)

  ;; Function to fill table $t1 using a function reference from table $t2
  (func (export "fill") (param $i i32) (param $r i32) (param $n i32)
    (local $ref funcref)
    (local.set $ref (table.get $t1 (local.get $r)))
    (table.fill $t2 (local.get $i) (local.get $ref) (local.get $n))
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8716000000         	ja	0x31
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8716000000         	ja	0x31
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8716000000         	ja	0x31
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c340000000       	addq	$0x40, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f87fd000000         	ja	0x118
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec28             	subq	$0x28, %rsp
;;      	 48897c2420           	movq	%rdi, 0x20(%rsp)
;;      	 4889742418           	movq	%rsi, 0x18(%rsp)
;;      	 89542414             	movl	%edx, 0x14(%rsp)
;;      	 894c2410             	movl	%ecx, 0x10(%rsp)
;;      	 448944240c           	movl	%r8d, 0xc(%rsp)
;;      	 c744240800000000     	movl	$0, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 448b5c2410           	movl	0x10(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c89f2               	movq	%r14, %rdx
;;      	 8b5a50               	movl	0x50(%rdx), %ebx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 0f83af000000         	jae	0x11a
;;   6b:	 4189cb               	movl	%ecx, %r11d
;;      	 4d6bdb08             	imulq	$8, %r11, %r11
;;      	 488b5248             	movq	0x48(%rdx), %rdx
;;      	 4889d6               	movq	%rdx, %rsi
;;      	 4c01da               	addq	%r11, %rdx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 480f43d6             	cmovaeq	%rsi, %rdx
;;      	 488b02               	movq	(%rdx), %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f852e000000         	jne	0xbc
;;   8e:	 4883ec04             	subq	$4, %rsp
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 4883ec04             	subq	$4, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 8b542404             	movl	4(%rsp), %edx
;;      	 e800000000           	callq	0xaa
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742420           	movq	0x20(%rsp), %r14
;;      	 e904000000           	jmp	0xc0
;;   bc:	 4883e0fe             	andq	$0xfffffffffffffffe, %rax
;;      	 4889442404           	movq	%rax, 4(%rsp)
;;      	 448b5c2414           	movl	0x14(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 4c8b5c2408           	movq	8(%rsp), %r11
;;      	 4153                 	pushq	%r11
;;      	 448b5c2418           	movl	0x18(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be01000000           	movl	$1, %esi
;;      	 8b542414             	movl	0x14(%rsp), %edx
;;      	 488b4c240c           	movq	0xc(%rsp), %rcx
;;      	 448b442408           	movl	8(%rsp), %r8d
;;      	 e800000000           	callq	0x105
;;      	 4883c408             	addq	$8, %rsp
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 4c8b742420           	movq	0x20(%rsp), %r14
;;      	 4883c428             	addq	$0x28, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;  118:	 0f0b                 	ud2	
;;  11a:	 0f0b                 	ud2	
