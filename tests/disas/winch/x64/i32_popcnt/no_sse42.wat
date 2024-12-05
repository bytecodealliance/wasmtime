;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_popcnt"]

(module
    (func (result i32)
      i32.const 3
      i32.popcnt
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $3, %eax
;;       movl    %eax, %ecx
;;       shrl    $1, %eax
;;       andl    $0x55555555, %eax
;;       subl    %eax, %ecx
;;       movl    %ecx, %eax
;;       movl    $0x33333333, %r11d
;;       andl    %r11d, %eax
;;       shrl    $2, %ecx
;;       andl    %r11d, %ecx
;;       addl    %eax, %ecx
;;       movl    %ecx, %eax
;;       shrl    $4, %eax
;;       addl    %ecx, %eax
;;       andl    $0xf0f0f0f, %eax
;;       imull   $0x1010101, %eax, %eax
;;       shrl    $0x18, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   6d: ud2
