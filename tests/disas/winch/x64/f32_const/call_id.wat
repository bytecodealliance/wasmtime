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
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x45
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movss   %xmm0, 0xc(%rsp)
;;       movss   0xc(%rsp), %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   45: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xa0
;;   6c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movss   0x1b(%rip), %xmm0
;;       callq   0
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   a0: ud2
;;   a2: addb    %al, (%rax)
;;   a4: addb    %al, (%rax)
;;   a6: addb    %al, (%rax)
;;   a8: retq
;;   a9: cmc
;;   aa: testb   $0x3f, %al
