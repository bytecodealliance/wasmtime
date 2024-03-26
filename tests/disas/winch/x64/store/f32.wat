;;! target = "x86_64"
;;! test = "winch"

(module
  (memory (data "\00\00\a0\7f"))
  (func (export "f32.store") (f32.store (i32.const 0) (f32.const nan:0x200000)))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x49
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x1d(%rip), %xmm0
;;   33: movl    $0, %eax
;;   38: movq    0x50(%r14), %rcx
;;   3c: addq    %rax, %rcx
;;   3f: movss   %xmm0, (%rcx)
;;   43: addq    $0x10, %rsp
;;   47: popq    %rbp
;;   48: retq
;;   49: ud2
;;   4b: addb    %al, (%rax)
;;   4d: addb    %al, (%rax)
;;   4f: addb    %al, (%rax)
