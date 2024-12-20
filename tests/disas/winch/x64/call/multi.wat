;;! target = "x86_64"
;;! test = "winch"
(module
  (func $multi (result i32 i32)
        i32.const 1
        i32.const 2)

  (func $start
        call $multi
        drop
        drop)
)
;; wasm[0]::function[0]::multi:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x24, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x58
;;   1c: movq    %rsi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rsi, 0x18(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movl    $2, %eax
;;       subq    $4, %rsp
;;       movl    $1, (%rsp)
;;       movq    0xc(%rsp), %rcx
;;       movl    (%rsp), %r11d
;;       addq    $4, %rsp
;;       movl    %r11d, (%rcx)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   58: ud2
;;
;; wasm[0]::function[1]::start:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xb7
;;   7c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       subq    $4, %rsp
;;       subq    $0xc, %rsp
;;       movq    %r14, %rsi
;;       movq    %r14, %rdx
;;       leaq    0xc(%rsp), %rdi
;;       callq   0
;;       addq    $0xc, %rsp
;;       movq    0xc(%rsp), %r14
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   b7: ud2
