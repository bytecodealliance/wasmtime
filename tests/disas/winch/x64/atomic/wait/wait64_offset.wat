;;! target = "x86_64"
;;! test = "winch"

(module
  (memory 1 1 shared)
  (func (export "_start") (result i32)
        (memory.atomic.wait64 offset=8
          (i32.const 4)
          (i64.const 0)
          (i64.const -1))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x79
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $18446744073709551615, %rax
;;       movq    $0, %rcx
;;       movl    $4, %edx
;;       addq    $8, %rdx
;;       pushq   %rdx
;;       pushq   %rcx
;;       pushq   %rax
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movq    0x18(%rsp), %rdx
;;       movq    0x10(%rsp), %rcx
;;       movq    8(%rsp), %r8
;;       callq   0x19c
;;       addq    $8, %rsp
;;       addq    $0x18, %rsp
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   79: ud2
