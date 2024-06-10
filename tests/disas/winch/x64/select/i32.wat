;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "select-i32") (param i32 i32 i32) (result i32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x53
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    %ecx, 8(%rsp)
;;       movl    %r8d, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       movl    8(%rsp), %ecx
;;       movl    0xc(%rsp), %edx
;;       cmpl    $0, %eax
;;       cmovnel %edx, %ecx
;;       movl    %ecx, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   53: ud2
