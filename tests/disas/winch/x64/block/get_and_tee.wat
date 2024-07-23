;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (param i32) (result i32)
    local.get 0
    block
    end
    local.tee 0
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x24, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4e
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movl    %eax, 0xc(%rsp)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   4e: ud2
