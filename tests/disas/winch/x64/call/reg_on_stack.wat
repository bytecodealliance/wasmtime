;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "") (param i32) (result i32)
    local.get 0
    i32.const 1
    call 0
    i32.const 1
    call 0
    br_if 0 (;@0;)
    unreachable
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xac
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    $1, %edx
;;       callq   0
;;       addq    $0xc, %rsp
;;       movq    0x1c(%rsp), %r14
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    $1, %edx
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x20(%rsp), %r14
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       testl   %ecx, %ecx
;;       je      0xa4
;;   9b: addq    $4, %rsp
;;       jmp     0xa6
;;   a4: ud2
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   ac: ud2
