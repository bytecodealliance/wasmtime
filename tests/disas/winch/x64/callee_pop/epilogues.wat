;;! target = "x86_64"
;;! test = "winch"

;; Pin the ordinary return, compact SP-relative return, and FP-relative
;; fallback. The last two differ by one local at the compact-frame boundary.
(module
  (func (export "no_stack_args") (result i64)
    i64.const 42)
  (func (export "compact") (param i64 i64 i64 i64 i64 i64 i64 i64) (result i64)
    (local i64 i64 i64 i64)
    local.get 0 local.get 7 i64.add)
  (func (export "fallback") (param i64 i64 i64 i64 i64 i64 i64 i64) (result i64)
    (local i64 i64 i64 i64 i64)
    local.get 0 local.get 7 i64.add))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0x2a, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3d: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x50, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xbd
;;   5c: movq    %rdi, %r14
;;       subq    $0x50, %rsp
;;       movq    %rdi, 0x48(%rsp)
;;       movq    %rsi, 0x40(%rsp)
;;       movq    %rdx, 0x38(%rsp)
;;       movq    %rcx, 0x30(%rsp)
;;       movq    %r8, 0x28(%rsp)
;;       movq    %r9, 0x20(%rsp)
;;       xorq    %r11, %r11
;;       movq    %r11, 0x18(%rsp)
;;       movq    %r11, 0x10(%rsp)
;;       movq    %r11, 8(%rsp)
;;       movq    %r11, (%rsp)
;;       movq    0x28(%rbp), %rax
;;       movq    0x38(%rsp), %rcx
;;       addq    %rax, %rcx
;;       movq    %rcx, %rax
;;       movq    0x58(%rsp), %r11
;;       movq    %r11, 0x78(%rsp)
;;       movq    0x50(%rsp), %rbp
;;       addq    $0x78, %rsp
;;       retq
;;   bd: ud2
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x60, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x143
;;   dc: movq    %rdi, %r14
;;       subq    $0x60, %rsp
;;       movq    %rdi, 0x58(%rsp)
;;       movq    %rsi, 0x50(%rsp)
;;       movq    %rdx, 0x48(%rsp)
;;       movq    %rcx, 0x40(%rsp)
;;       movq    %r8, 0x38(%rsp)
;;       movq    %r9, 0x30(%rsp)
;;       xorq    %r11, %r11
;;       movq    %r11, 0x28(%rsp)
;;       movq    %r11, 0x20(%rsp)
;;       movq    %r11, 0x18(%rsp)
;;       movq    %r11, 0x10(%rsp)
;;       movq    %r11, 8(%rsp)
;;       movq    0x28(%rbp), %rax
;;       movq    0x48(%rsp), %rcx
;;       addq    %rax, %rcx
;;       movq    %rcx, %rax
;;       movq    8(%rbp), %r11
;;       movq    %r11, 0x28(%rbp)
;;       leaq    0x28(%rbp), %r11
;;       movq    (%rbp), %rbp
;;       movq    %r11, %rsp
;;       retq
;;  143: ud2
