;;! target="x86_64"

(module
  (type $over-i32 (func (param i32) (result i32)))

  (table funcref
    (elem
      $fib-i32
    )
  )
  
  (func $fib-i32 (export "fib-i32") (type $over-i32)
    (if (result i32) (i32.le_u (local.get 0) (i32.const 1))
      (then (i32.const 1))
      (else
        (i32.add
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 2))
            (i32.const 0)
          )
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 1))
            (i32.const 0)
          )
        )
      )
    )
  )
)


;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c330000000       	addq	$0x30, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f87ad010000         	ja	0x1c8
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 83f801               	cmpl	$1, %eax
;;      	 b800000000           	movl	$0, %eax
;;      	 400f96c0             	setbe	%al
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f840a000000         	je	0x52
;;   48:	 b801000000           	movl	$1, %eax
;;      	 e970010000           	jmp	0x1c2
;;   52:	 8b442404             	movl	4(%rsp), %eax
;;      	 83e802               	subl	$2, %eax
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 b900000000           	movl	$0, %ecx
;;      	 4c89f2               	movq	%r14, %rdx
;;      	 8b5a50               	movl	0x50(%rdx), %ebx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 0f8357010000         	jae	0x1ca
;;   73:	 4189cb               	movl	%ecx, %r11d
;;      	 4d6bdb08             	imulq	$8, %r11, %r11
;;      	 488b5248             	movq	0x48(%rdx), %rdx
;;      	 4889d6               	movq	%rdx, %rsi
;;      	 4c01da               	addq	%r11, %rdx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 480f43d6             	cmovaeq	%rsi, %rdx
;;      	 488b02               	movq	(%rdx), %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f8525000000         	jne	0xbb
;;   96:	 4883ec04             	subq	$4, %rsp
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 8b1424               	movl	(%rsp), %edx
;;      	 e800000000           	callq	0xad
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742414           	movq	0x14(%rsp), %r14
;;      	 e904000000           	jmp	0xbf
;;   bb:	 4883e0fe             	andq	$0xfffffffffffffffe, %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f8404010000         	je	0x1cc
;;   c8:	 4d8b5e40             	movq	0x40(%r14), %r11
;;      	 418b0b               	movl	(%r11), %ecx
;;      	 8b5018               	movl	0x18(%rax), %edx
;;      	 39d1                 	cmpl	%edx, %ecx
;;      	 0f85f4000000         	jne	0x1ce
;;   da:	 50                   	pushq	%rax
;;      	 59                   	popq	%rcx
;;      	 4c8b4120             	movq	0x20(%rcx), %r8
;;      	 488b5910             	movq	0x10(%rcx), %rbx
;;      	 4883ec04             	subq	$4, %rsp
;;      	 4c89c7               	movq	%r8, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 8b542404             	movl	4(%rsp), %edx
;;      	 ffd3                 	callq	*%rbx
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 83e901               	subl	$1, %ecx
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 b900000000           	movl	$0, %ecx
;;      	 4c89f2               	movq	%r14, %rdx
;;      	 8b5a50               	movl	0x50(%rdx), %ebx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 0f83a7000000         	jae	0x1d0
;;  129:	 4189cb               	movl	%ecx, %r11d
;;      	 4d6bdb08             	imulq	$8, %r11, %r11
;;      	 488b5248             	movq	0x48(%rdx), %rdx
;;      	 4889d6               	movq	%rdx, %rsi
;;      	 4c01da               	addq	%r11, %rdx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 480f43d6             	cmovaeq	%rsi, %rdx
;;      	 488b02               	movq	(%rdx), %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f852e000000         	jne	0x17a
;;  14c:	 4883ec04             	subq	$4, %rsp
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 4883ec0c             	subq	$0xc, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 8b54240c             	movl	0xc(%rsp), %edx
;;      	 e800000000           	callq	0x168
;;      	 4883c40c             	addq	$0xc, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742418           	movq	0x18(%rsp), %r14
;;      	 e904000000           	jmp	0x17e
;;  17a:	 4883e0fe             	andq	$0xfffffffffffffffe, %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f844b000000         	je	0x1d2
;;  187:	 4d8b5e40             	movq	0x40(%r14), %r11
;;      	 418b0b               	movl	(%r11), %ecx
;;      	 8b5018               	movl	0x18(%rax), %edx
;;      	 39d1                 	cmpl	%edx, %ecx
;;      	 0f853b000000         	jne	0x1d4
;;  199:	 50                   	pushq	%rax
;;      	 59                   	popq	%rcx
;;      	 4c8b4120             	movq	0x20(%rcx), %r8
;;      	 488b5910             	movq	0x10(%rcx), %rbx
;;      	 4c89c7               	movq	%r8, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 8b1424               	movl	(%rsp), %edx
;;      	 ffd3                 	callq	*%rbx
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742414           	movq	0x14(%rsp), %r14
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 01c1                 	addl	%eax, %ecx
;;      	 89c8                 	movl	%ecx, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;  1c8:	 0f0b                 	ud2	
;;  1ca:	 0f0b                 	ud2	
;;  1cc:	 0f0b                 	ud2	
;;  1ce:	 0f0b                 	ud2	
;;  1d0:	 0f0b                 	ud2	
;;  1d2:	 0f0b                 	ud2	
;;  1d4:	 0f0b                 	ud2	
