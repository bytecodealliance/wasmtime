;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "select-i64") (param i64 i64 i32) (result i64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x59
;;   1b: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movq    %rdx, 0x18(%rsp)
;;       movq    %rcx, 0x10(%rsp)
;;       movl    %r8d, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movq    0x10(%rsp), %rcx
;;       movq    0x18(%rsp), %rdx
;;       cmpl    $0, %eax
;;       cmovneq %rdx, %rcx
;;       movq    %rcx, %rax
;;       addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;   59: ud2
