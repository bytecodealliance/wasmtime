;;! target = "x86_64"
;;! test = "winch"

(module
  (func $id-v128 (param v128) (result v128) (local.get 0))
  (func (export "a")
    (call $id-v128 (v128.const i64x2 1 2))
    drop
  )
)
;; wasm[0]::function[0]::id-v128:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3d
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   3d: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x8a
;;   5c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movdqu  0x16(%rip), %xmm0
;;       callq   0
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   8a: ud2
;;   8c: addb    %al, (%rax)
;;   8e: addb    %al, (%rax)
;;   90: addl    %eax, (%rax)
;;   92: addb    %al, (%rax)
;;   94: addb    %al, (%rax)
;;   96: addb    %al, (%rax)
;;   98: addb    (%rax), %al
;;   9a: addb    %al, (%rax)
;;   9c: addb    %al, (%rax)
;;   9e: addb    %al, (%rax)
