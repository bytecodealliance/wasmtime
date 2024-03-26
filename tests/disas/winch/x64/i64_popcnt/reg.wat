;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_popcnt", "-Ccranelift-has_sse42"]

(module
    (func (param i64) (result i64)
      local.get 0
      i64.popcnt
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3f
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    %rdx, (%rsp)
;;       movq    (%rsp), %rax
;;       popcntq %rax, %rax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   3f: ud2
