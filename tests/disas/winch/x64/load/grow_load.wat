;;! target = "x86_64"
;;! test = "winch"
(module
  (type (;0;) (func (param f32 f64 i64 i64 f64 f64 f32) (result f32 f64 f32)))
  (func (;0;) (type 0) (param f32 f64 i64 i64 f64 f64 f32) (result f32 f64 f32)
    global.get 1
    i32.eqz
    if ;; label = @1
      unreachable
    end
    global.get 1
    i32.const 1
    i32.sub
    global.set 1
    memory.size
    memory.grow
    i64.load8_s offset=143396
    (drop)
    (f32.const 0)
    (f64.const 0)
    (f32.const 0)
  )
  (memory (;1;) 10 10)
  (global (;0;) f32 f32.const 0x1.d6a0d6p+87 (;=284477330000000000000000000;))
  (global (;1;) (mut i32) i32.const 1000)
  (export "main" (func 0))
  (export "0" (memory 0))
  (export "1" (global 0))
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c360000000       	addq	$0x60, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f87ed000000         	ja	0x108
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec50             	subq	$0x50, %rsp
;;      	 48897c2448           	movq	%rdi, 0x48(%rsp)
;;      	 4889742440           	movq	%rsi, 0x40(%rsp)
;;      	 f30f1144243c         	movss	%xmm0, 0x3c(%rsp)
;;      	 f20f114c2430         	movsd	%xmm1, 0x30(%rsp)
;;      	 4889542428           	movq	%rdx, 0x28(%rsp)
;;      	 48894c2420           	movq	%rcx, 0x20(%rsp)
;;      	 f20f11542418         	movsd	%xmm2, 0x18(%rsp)
;;      	 f20f115c2410         	movsd	%xmm3, 0x10(%rsp)
;;      	 f30f1164240c         	movss	%xmm4, 0xc(%rsp)
;;      	 4c890424             	movq	%r8, (%rsp)
;;      	 418b4670             	movl	0x70(%r14), %eax
;;      	 83f800               	cmpl	$0, %eax
;;      	 b800000000           	movl	$0, %eax
;;      	 400f94c0             	sete	%al
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8402000000         	je	0x72
;;   70:	 0f0b                 	ud2	
;;      	 418b4670             	movl	0x70(%r14), %eax
;;      	 83e801               	subl	$1, %eax
;;      	 41894670             	movl	%eax, 0x70(%r14)
;;      	 498b4658             	movq	0x58(%r14), %rax
;;      	 c1e810               	shrl	$0x10, %eax
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 4883ec0c             	subq	$0xc, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 8b74240c             	movl	0xc(%rsp), %esi
;;      	 ba00000000           	movl	$0, %edx
;;      	 e800000000           	callq	0xa0
;;      	 4883c40c             	addq	$0xc, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742448           	movq	0x48(%rsp), %r14
;;      	 89c0                 	movl	%eax, %eax
;;      	 498b4e50             	movq	0x50(%r14), %rcx
;;      	 4801c1               	addq	%rax, %rcx
;;      	 4881c124300200       	addq	$0x23024, %rcx
;;      	 480fbe01             	movsbq	(%rcx), %rax
;;      	 f30f100547000000     	movss	0x47(%rip), %xmm0
;;      	 4883ec0c             	subq	$0xc, %rsp
;;      	 f2440f103d42000000   	
;; 				movsd	0x42(%rip), %xmm15
;;      	 f2440f113c24         	movsd	%xmm15, (%rsp)
;;      	 f3440f103d2b000000   	
;; 				movss	0x2b(%rip), %xmm15
;;      	 f3440f117c2408       	movss	%xmm15, 8(%rsp)
;;      	 488b44240c           	movq	0xc(%rsp), %rax
;;      	 415b                 	popq	%r11
;;      	 4c8918               	movq	%r11, (%rax)
;;      	 448b1c24             	movl	(%rsp), %r11d
;;      	 4883c404             	addq	$4, %rsp
;;      	 44895808             	movl	%r11d, 8(%rax)
;;      	 4883c450             	addq	$0x50, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;  108:	 0f0b                 	ud2	
;;  10a:	 0000                 	addb	%al, (%rax)
;;  10c:	 0000                 	addb	%al, (%rax)
;;  10e:	 0000                 	addb	%al, (%rax)
;;  110:	 0000                 	addb	%al, (%rax)
;;  112:	 0000                 	addb	%al, (%rax)
;;  114:	 0000                 	addb	%al, (%rax)
;;  116:	 0000                 	addb	%al, (%rax)
;;  118:	 0000                 	addb	%al, (%rax)
;;  11a:	 0000                 	addb	%al, (%rax)
;;  11c:	 0000                 	addb	%al, (%rax)
;;  11e:	 0000                 	addb	%al, (%rax)
