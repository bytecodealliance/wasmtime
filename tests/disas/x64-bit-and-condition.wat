;;! target = 'x86_64'
;;! test = 'compile'

(module
  (func $if_b20 (param i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )
  (func $select_b20 (param i32 i32 i32) (result i32)
    local.get 1
    local.get 2
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    select
  )
  (func $eqz_b20 (param i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    i32.eqz
  )
)
;; wasm[0]::function[0]::if_b20:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $0x100000, %edx
;;       jne     0x1a
;;   10: movl    $0xc8, %eax
;;       jmp     0x1f
;;   1a: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::select_b20:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $0x100000, %edx
;;       movq    %r8, %rax
;;       cmovnel %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::eqz_b20:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       testl   $0x100000, %edx
;;       sete    %r8b
;;       movzbl  %r8b, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
