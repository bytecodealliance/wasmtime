;;! target = "x86_64"
;;! test = "winch"

(module
  (func $id-f64 (param f64) (result f64) (local.get 0))
  (func (export "type-first-f64") (result f64) (call $id-f64 (f64.const 1.32)))
)
;; wasm[0]::function[0]::id-f64:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3f
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   8(%rsp), %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   3f: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x9a
;;   6c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movsd   0x16(%rip), %xmm0
;;       callq   0
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   9a: ud2
;;   9c: addb    %al, (%rax)
;;   9e: addb    %al, (%rax)
