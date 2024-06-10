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
;;       addq    $0x24, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xa4
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    $1, %edx
;;       callq   0
;;       addq    $4, %rsp
;;       movq    0x14(%rsp), %r14
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    $1, %edx
;;       callq   0
;;       movq    0x18(%rsp), %r14
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       testl   %ecx, %ecx
;;       je      0x9c
;;   93: addq    $4, %rsp
;;       jmp     0x9e
;;   9c: ud2
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   a4: ud2
