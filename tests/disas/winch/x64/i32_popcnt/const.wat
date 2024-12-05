;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has-popcnt", "-Ccranelift-has-sse42"]

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
;;       ja      0x3b
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $3, %eax
;;       popcntl %eax, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3b: ud2
