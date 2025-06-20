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
;;       ja      0x43
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   43: ud2
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
;;       movdqu  0x23(%rip), %xmm0
;;       callq   0
;;       movq    8(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   a0: ud2
;;   a2: addb    %al, (%rax)
;;   a4: addb    %al, (%rax)
;;   a6: addb    %al, (%rax)
;;   a8: addb    %al, (%rax)
;;   aa: addb    %al, (%rax)
;;   ac: addb    %al, (%rax)
;;   ae: addb    %al, (%rax)
;;   b0: addl    %eax, (%rax)
;;   b2: addb    %al, (%rax)
;;   b4: addb    %al, (%rax)
;;   b6: addb    %al, (%rax)
;;   b8: addb    (%rax), %al
;;   ba: addb    %al, (%rax)
;;   bc: addb    %al, (%rax)
;;   be: addb    %al, (%rax)
