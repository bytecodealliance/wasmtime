;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "")
    (local i32)
    local.get 0
    if
      local.get 0
      block
      end
      unreachable
    else
      nop
    end
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x1c, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x55
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movl    4(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x4f
;;   40: movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       ud2
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   55: ud2
