;;! target = "x86_64"
;;! test = "winch"

;; Nonconstant arguments force spills. Keep a value live below the arguments,
;; then consume both register results and a separate stack-result area.
(module
  (type $single (func (param i64 i64 i64 i64 i64 i64 i64 i64 i64 i64) (result i64)))
  (func $single (type $single) local.get 0 local.get 9 i64.add)
  (func $multi (param i64 i64 i64 i64 i64 i64 i64 i64 i64 i64) (result i64 i64 f64)
    local.get 0 local.get 9 f64.const 3.5)
  (table funcref (elem $single))
  (func (export "direct") (param i64) (result i64)
    local.get 0
    local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0
    call $single i64.add)
  (func (export "indirect") (param i64) (result i64)
    local.get 0
    local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0
    i32.const 0 call_indirect (type $single) i64.add)
  (func (export "stack_results") (param i64) (result i64)
    local.get 0
    local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0
    call $multi i64.trunc_f64_s i64.add i64.add i64.add))
;; wasm[0]::function[0]::single:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x66
;;   1c: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movq    %rdx, 0x18(%rsp)
;;       movq    %rcx, 0x10(%rsp)
;;       movq    %r8, 8(%rsp)
;;       movq    %r9, (%rsp)
;;       movq    0x38(%rbp), %rax
;;       movq    0x18(%rsp), %rcx
;;       addq    %rax, %rcx
;;       movq    %rcx, %rax
;;       movq    0x38(%rsp), %r11
;;       movq    %r11, 0x68(%rsp)
;;       movq    0x30(%rsp), %rbp
;;       addq    $0x68, %rsp
;;       retq
;;   66: ud2
;;
;; wasm[0]::function[1]::multi:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x50, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xed
;;   8c: movq    %rsi, %r14
;;       subq    $0x40, %rsp
;;       movq    %rsi, 0x38(%rsp)
;;       movq    %rdx, 0x30(%rsp)
;;       movq    %rcx, 0x28(%rsp)
;;       movq    %r8, 0x20(%rsp)
;;       movq    %r9, 0x18(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movsd   0x34(%rip), %xmm0
;;       movq    0x28(%rsp), %r11
;;       pushq   %r11
;;       movq    0x40(%rbp), %r11
;;       pushq   %r11
;;       movq    0x18(%rsp), %rax
;;       popq    %r11
;;       movq    %r11, (%rax)
;;       popq    %r11
;;       movq    %r11, 8(%rax)
;;       movq    8(%rbp), %r11
;;       movq    %r11, 0x48(%rbp)
;;       leaq    0x48(%rbp), %r11
;;       movq    (%rbp), %rbp
;;       movq    %r11, %rsp
;;       retq
;;   ed: ud2
;;   ef: addb    %al, (%rax)
;;   f1: addb    %al, (%rax)
;;   f3: addb    %al, (%rax)
;;   f5: addb    %cl, (%rax, %rax, 2)
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0xb0, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x202
;;  11c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movq    8(%rsp), %r11
;;       pushq   %r11
;;       movq    0x10(%rsp), %r11
;;       pushq   %r11
;;       movq    0x18(%rsp), %r11
;;       pushq   %r11
;;       movq    0x20(%rsp), %r11
;;       pushq   %r11
;;       movq    0x28(%rsp), %r11
;;       pushq   %r11
;;       movq    0x30(%rsp), %r11
;;       pushq   %r11
;;       movq    0x38(%rsp), %r11
;;       pushq   %r11
;;       movq    0x40(%rsp), %r11
;;       pushq   %r11
;;       movq    0x48(%rsp), %r11
;;       pushq   %r11
;;       movq    0x50(%rsp), %r11
;;       pushq   %r11
;;       movq    0x58(%rsp), %r11
;;       pushq   %r11
;;       subq    $0x38, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movq    0x80(%rsp), %rdx
;;       movq    0x78(%rsp), %rcx
;;       movq    0x70(%rsp), %r8
;;       movq    0x68(%rsp), %r9
;;       movq    0x60(%rsp), %r11
;;       movq    %r11, (%rsp)
;;       movq    0x58(%rsp), %r11
;;       movq    %r11, 8(%rsp)
;;       movq    0x50(%rsp), %r11
;;       movq    %r11, 0x10(%rsp)
;;       movq    0x48(%rsp), %r11
;;       movq    %r11, 0x18(%rsp)
;;       movq    0x40(%rsp), %r11
;;       movq    %r11, 0x20(%rsp)
;;       movq    0x38(%rsp), %r11
;;       movq    %r11, 0x28(%rsp)
;;       callq   0
;;       addq    $0x58, %rsp
;;       movq    0x20(%rsp), %r14
;;       popq    %rcx
;;       addq    %rax, %rcx
;;       movq    %rcx, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  202: ud2
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0xb0, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3aa
;;  22c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movq    8(%rsp), %r11
;;       pushq   %r11
;;       movq    0x10(%rsp), %r11
;;       pushq   %r11
;;       movq    0x18(%rsp), %r11
;;       pushq   %r11
;;       movq    0x20(%rsp), %r11
;;       pushq   %r11
;;       movq    0x28(%rsp), %r11
;;       pushq   %r11
;;       movq    0x30(%rsp), %r11
;;       pushq   %r11
;;       movq    0x38(%rsp), %r11
;;       pushq   %r11
;;       movq    0x40(%rsp), %r11
;;       pushq   %r11
;;       movq    0x48(%rsp), %r11
;;       pushq   %r11
;;       movq    0x50(%rsp), %r11
;;       pushq   %r11
;;       movq    0x58(%rsp), %r11
;;       pushq   %r11
;;       movl    $0, %ecx
;;       movq    %r14, %rdx
;;       movq    0x38(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x3ac
;;  2a7: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x30(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x301
;;  2ce: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    4(%rsp), %edx
;;       callq   0xa7a
;;       addq    $8, %rsp
;;       movq    0x70(%rsp), %r14
;;       jmp     0x307
;;  301: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0x3ae
;;  310: movq    0x28(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0x3b0
;;  322: pushq   %rax
;;       popq    %rbx
;;       movq    0x18(%rbx), %r12
;;       movq    8(%rbx), %r10
;;       subq    $0x38, %rsp
;;       movq    %r12, %rdi
;;       movq    %r14, %rsi
;;       movq    0x80(%rsp), %rdx
;;       movq    0x78(%rsp), %rcx
;;       movq    0x70(%rsp), %r8
;;       movq    0x68(%rsp), %r9
;;       movq    0x60(%rsp), %r11
;;       movq    %r11, (%rsp)
;;       movq    0x58(%rsp), %r11
;;       movq    %r11, 8(%rsp)
;;       movq    0x50(%rsp), %r11
;;       movq    %r11, 0x10(%rsp)
;;       movq    0x48(%rsp), %r11
;;       movq    %r11, 0x18(%rsp)
;;       movq    0x40(%rsp), %r11
;;       movq    %r11, 0x20(%rsp)
;;       movq    0x38(%rsp), %r11
;;       movq    %r11, 0x28(%rsp)
;;       callq   *%r10
;;       addq    $0x58, %rsp
;;       movq    0x20(%rsp), %r14
;;       popq    %rcx
;;       addq    %rax, %rcx
;;       movq    %rcx, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  3aa: ud2
;;  3ac: ud2
;;  3ae: ud2
;;  3b0: ud2
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0xd0, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x544
;;  3dc: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movq    8(%rsp), %r11
;;       pushq   %r11
;;       movq    0x10(%rsp), %r11
;;       pushq   %r11
;;       movq    0x18(%rsp), %r11
;;       pushq   %r11
;;       movq    0x20(%rsp), %r11
;;       pushq   %r11
;;       movq    0x28(%rsp), %r11
;;       pushq   %r11
;;       movq    0x30(%rsp), %r11
;;       pushq   %r11
;;       movq    0x38(%rsp), %r11
;;       pushq   %r11
;;       movq    0x40(%rsp), %r11
;;       pushq   %r11
;;       movq    0x48(%rsp), %r11
;;       pushq   %r11
;;       movq    0x50(%rsp), %r11
;;       pushq   %r11
;;       movq    0x58(%rsp), %r11
;;       pushq   %r11
;;       subq    $0x10, %rsp
;;       subq    $0x48, %rsp
;;       movq    %r14, %rsi
;;       movq    %r14, %rdx
;;       movq    0xa0(%rsp), %rcx
;;       movq    0x98(%rsp), %r8
;;       movq    0x90(%rsp), %r9
;;       movq    0x88(%rsp), %r11
;;       movq    %r11, (%rsp)
;;       movq    0x80(%rsp), %r11
;;       movq    %r11, 8(%rsp)
;;       movq    0x78(%rsp), %r11
;;       movq    %r11, 0x10(%rsp)
;;       movq    0x70(%rsp), %r11
;;       movq    %r11, 0x18(%rsp)
;;       movq    0x68(%rsp), %r11
;;       movq    %r11, 0x20(%rsp)
;;       movq    0x60(%rsp), %r11
;;       movq    %r11, 0x28(%rsp)
;;       movq    0x58(%rsp), %r11
;;       movq    %r11, 0x30(%rsp)
;;       leaq    0x48(%rsp), %rdi
;;       callq   0x70
;;       addq    $8, %rsp
;;       movq    8(%rsp), %r11
;;       movq    %r11, 0x58(%rsp)
;;       movq    (%rsp), %r11
;;       movq    %r11, 0x50(%rsp)
;;       addq    $0x50, %rsp
;;       movq    0x30(%rsp), %r14
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x52c
;;  4f8: ucomisd %xmm0, %xmm0
;;       jp      0x546
;;  502: movabsq $14114281232179134464, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jb      0x548
;;  51c: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x54a
;;  52c: popq    %rcx
;;       addq    %rax, %rcx
;;       popq    %rax
;;       addq    %rcx, %rax
;;       popq    %rcx
;;       addq    %rax, %rcx
;;       movq    %rcx, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  544: ud2
;;  546: ud2
;;  548: ud2
;;  54a: ud2
