;;! target = "x86_64"
;;! test = "winch"

(module
  (func $main (result i32)
    (local $var i32)
    (call $product (i32.const 20) (i32.const 80))
    (local.set $var (i32.const 2))
    (local.get $var)
    (i32.div_u))

  (func $product (param i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.mul))
)
;; wasm[0]::function[0]::main:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x8b
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    $0x14, %edx
;;       movl    $0x50, %ecx
;;       callq   0x90
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $2, %ecx
;;       movl    %ecx, 4(%rsp)
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    8(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       xorl    %edx, %edx
;;       divl    %ecx
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   8b: ud2
;;
;; wasm[0]::function[1]::product:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xd5
;;   ab: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    (%rsp), %eax
;;       movl    4(%rsp), %ecx
;;       imull   %eax, %ecx
;;       movl    %ecx, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   d5: ud2
