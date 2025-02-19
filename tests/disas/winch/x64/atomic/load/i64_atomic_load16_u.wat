;;! target = "x86_64"
;;! test = "winch"

(module 
  (memory (data "\00\00\00\00\00\00\f4\7f"))

  (func (result i64)
        (i64.atomic.load16_u
          (i32.const 0))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x55
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0, %eax
;;       andw    $1, %ax
;;       cmpw    $0, %ax
;;       jne     0x57
;;   3f: movl    $0, %eax
;;       movq    0x58(%r14), %rcx
;;       addq    %rax, %rcx
;;       movzwq  (%rcx), %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   55: ud2
;;   57: ud2
