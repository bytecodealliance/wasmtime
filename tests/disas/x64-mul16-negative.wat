;;! target = 'x86_64'
;;! test = 'compile'

(module
  (func (export "mul16") (param i32) (result i32)
    local.get 0
    i32.const -7937
    i32.mul
    i32.extend16_s
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       imulw   $0xe0ff, %dx, %dx
;;       movswl  %dx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
