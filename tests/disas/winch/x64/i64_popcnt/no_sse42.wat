;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_popcnt"]

(module
    (func (result i64)
      i64.const 3
      i64.popcnt
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x93
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $3, %rax
;;       movq    %rax, %rcx
;;       shrq    $1, %rax
;;       movabsq $0x5555555555555555, %r11
;;       andq    %r11, %rax
;;       subq    %rax, %rcx
;;       movq    %rcx, %rax
;;       movabsq $0x3333333333333333, %r11
;;       andq    %r11, %rax
;;       shrq    $2, %rcx
;;       andq    %r11, %rcx
;;       addq    %rax, %rcx
;;       movq    %rcx, %rax
;;       shrq    $4, %rax
;;       addq    %rcx, %rax
;;       movabsq $0xf0f0f0f0f0f0f0f, %r11
;;       andq    %r11, %rax
;;       movabsq $0x101010101010101, %r11
;;       imulq   %r11, %rax
;;       shrq    $0x38, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   93: ud2
