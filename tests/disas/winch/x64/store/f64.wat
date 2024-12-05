;;! target = "x86_64"
;;! test = "winch"

(module
  (memory (data "\00\00\00\00\00\00\f4\7f"))
  (func (export "f64.store") (f64.store (i32.const 0) (f64.const nan:0x4000000000000)))
)

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x1c(%rip), %xmm0
;;       movl    $0, %eax
;;       movq    0x60(%r14), %rcx
;;       addq    %rax, %rcx
;;       movsd   %xmm0, (%rcx)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4a: ud2
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: addb    %al, (%rax)
;;   52: addb    %al, (%rax)
;;   54: addb    %al, (%rax)
;;   56: hlt
