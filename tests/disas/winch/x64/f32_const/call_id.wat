;;! target = "x86_64"
;;! test = "winch"

(module
  (func $id-f32 (param f32) (result f32) (local.get 0))
  (func (export "type-first-f32") (result f32) (call $id-f32 (f32.const 1.32)))
)
;; wasm[0]::function[0]::id-f32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3e
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movss   %xmm0, 4(%rsp)
;;       movss   4(%rsp), %xmm0
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   3e: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x89
;;   5b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movss   0x17(%rip), %xmm0
;;       callq   0
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   89: ud2
;;   8b: addb    %al, (%rax)
;;   8d: addb    %al, (%rax)
;;   8f: addb    %al, %bl
;;   91: cmc
;;   92: testb   $0x3f, %al
