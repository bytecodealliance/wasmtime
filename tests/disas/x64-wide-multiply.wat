;;! test = "compile"
;;! target = "x86_64"

(module
  (func $smulhi (param i32 i32) (result i32)
    local.get 0
    i64.extend_i32_s
    local.get 1
    i64.extend_i32_s
    i64.mul
    i64.const 32
    i64.shr_s
    i32.wrap_i64
  )

  (func $umulhi (param i32 i32) (result i32)
    local.get 0
    i64.extend_i32_u
    local.get 1
    i64.extend_i32_u
    i64.mul
    i64.const 32
    i64.shr_s
    i32.wrap_i64
  )
)
;; wasm[0]::function[0]::smulhi:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       imull   %ecx
;;       movq    %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::umulhi:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       mull    %ecx
;;       movq    %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
