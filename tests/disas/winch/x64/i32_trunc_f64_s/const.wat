;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f875f000000         	ja	0x7a
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f100555000000     	movsd	0x55(%rip), %xmm0
;;      	 f20f2cc0             	cvttsd2si	%xmm0, %eax
;;      	 83f801               	cmpl	$1, %eax
;;      	 0f8134000000         	jno	0x74
;;   40:	 660f2ec0             	ucomisd	%xmm0, %xmm0
;;      	 0f8a32000000         	jp	0x7c
;;   4a:	 49bb000020000000e0c1 	
;; 				movabsq	$13970166044105375744, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f2ec7           	ucomisd	%xmm15, %xmm0
;;      	 0f861a000000         	jbe	0x7e
;;   64:	 66450f57ff           	xorpd	%xmm15, %xmm15
;;      	 66440f2ef8           	ucomisd	%xmm0, %xmm15
;;      	 0f820c000000         	jb	0x80
;;   74:	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   7a:	 0f0b                 	ud2	
;;   7c:	 0f0b                 	ud2	
;;   7e:	 0f0b                 	ud2	
;;   80:	 0f0b                 	ud2	
;;   82:	 0000                 	addb	%al, (%rax)
;;   84:	 0000                 	addb	%al, (%rax)
;;   86:	 0000                 	addb	%al, (%rax)
;;   88:	 0000                 	addb	%al, (%rax)
;;   8a:	 0000                 	addb	%al, (%rax)
;;   8c:	 0000                 	addb	%al, (%rax)
