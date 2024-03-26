;;! target = "x86_64"

(module
    (func (result f32)
        f64.const 1.0
        f32.demote_f64
        block
        end
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c314000000       	addq	$0x14, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8734000000         	ja	0x4f
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f100525000000     	movsd	0x25(%rip), %xmm0
;;      	 f20f5ac0             	cvtsd2ss	%xmm0, %xmm0
;;      	 4883ec04             	subq	$4, %rsp
;;      	 f30f110424           	movss	%xmm0, (%rsp)
;;      	 f30f100424           	movss	(%rsp), %xmm0
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4f:	 0f0b                 	ud2	
;;   51:	 0000                 	addb	%al, (%rax)
;;   53:	 0000                 	addb	%al, (%rax)
;;   55:	 0000                 	addb	%al, (%rax)
;;   57:	 0000                 	addb	%al, (%rax)
;;   59:	 0000                 	addb	%al, (%rax)
;;   5b:	 0000                 	addb	%al, (%rax)
;;   5d:	 00f0                 	addb	%dh, %al
