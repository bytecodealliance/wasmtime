;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-Wfuel=1"
(module
  (import "" "" (func))
  (func (export "")
        call 0
        call $other)
  (func $other))
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x9f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    8(%r14), %rax
;;       movq    (%rax), %r11
;;       addq    $1, %r11
;;       movq    %r11, (%rax)
;;       movq    8(%r14), %rcx
;;       movq    (%rcx), %rcx
;;       cmpq    $0, %rcx
;;       jl      0x58
;;   4b: movq    %r14, %rdi
;;       callq   0x201
;;       movq    8(%rsp), %r14
;;       movq    8(%r14), %rax
;;       movq    (%rax), %r11
;;       addq    $1, %r11
;;       movq    %r11, (%rax)
;;       movq    0x60(%r14), %rcx
;;       movq    0x50(%r14), %rax
;;       movq    %rcx, %rdi
;;       movq    %r14, %rsi
;;       callq   *%rax
;;       movq    8(%rsp), %r14
;;       movq    8(%r14), %rax
;;       movq    (%rax), %r11
;;       addq    $1, %r11
;;       movq    %r11, (%rax)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0xb0
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   9f: ud2
;;
;; wasm[0]::function[2]::other:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x10e
;;   cc: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    8(%r14), %rax
;;       movq    (%rax), %r11
;;       addq    $1, %r11
;;       movq    %r11, (%rax)
;;       movq    8(%r14), %rcx
;;       movq    (%rcx), %rcx
;;       cmpq    $0, %rcx
;;       jl      0x108
;;   fb: movq    %r14, %rdi
;;       callq   0x201
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  10e: ud2
