;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8763000000         	ja	0x7e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f30f100d55000000     	movss	0x55(%rip), %xmm1
;;      	 41bb0000004f         	movl	$0x4f000000, %r11d
;;      	 66450f6efb           	movd	%r11d, %xmm15
;;      	 410f2ecf             	ucomiss	%xmm15, %xmm1
;;      	 0f8315000000         	jae	0x5d
;;      	 0f8a32000000         	jp	0x80
;;   4e:	 f30f2cc1             	cvttss2si	%xmm1, %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8d1d000000         	jge	0x78
;;   5b:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 f3410f5cc7           	subss	%xmm15, %xmm0
;;      	 f30f2cc0             	cvttss2si	%xmm0, %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 0f8c10000000         	jl	0x82
;;   72:	 81c000000080         	addl	$0x80000000, %eax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   7e:	 0f0b                 	ud2	
;;   80:	 0f0b                 	ud2	
;;   82:	 0f0b                 	ud2	
;;   84:	 0000                 	addb	%al, (%rax)
;;   86:	 0000                 	addb	%al, (%rax)
;;   88:	 0000                 	addb	%al, (%rax)
