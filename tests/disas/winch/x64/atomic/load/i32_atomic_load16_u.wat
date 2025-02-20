;;! target = "x86_64"
;;! test = "winch"

(module 
  (import "env" "memory" (memory 1 1 shared))
  (func (param $foo i32) (result i32)
        (i32.atomic.load16_u
          (local.get $foo))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5b
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       andw    $1, %ax
;;       cmpw    $0, %ax
;;       jne     0x5d
;;   43: movl    0xc(%rsp), %eax
;;       movq    0x50(%r14), %r11
;;       movq    (%r11), %rcx
;;       addq    %rax, %rcx
;;       movzwq  (%rcx), %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   5b: ud2
;;   5d: ud2
