;;! target = "x86_64"
;;! test = "compile"

(module
  (func $i32_21 (param i32 i32) (result i32)
    local.get 0
    i32.const 11
    i32.shl
    local.get 1
    i32.const 21
    i32.shr_u
    i32.or)
  (func $i32_21_swapped (param i32 i32) (result i32)
    local.get 1
    i32.const 21
    i32.shr_u
    local.get 0
    i32.const 11
    i32.shl
    i32.or)
  (func $i32_11 (param i32 i32) (result i32)
    local.get 0
    i32.const 21
    i32.shl
    local.get 1
    i32.const 11
    i32.shr_u
    i32.or)

  (func $i64_21 (param i64 i64) (result i64)
    local.get 0
    i64.const 43
    i64.shl
    local.get 1
    i64.const 21
    i64.shr_u
    i64.or)
  (func $i64_21_swapped (param i64 i64) (result i64)
    local.get 1
    i64.const 21
    i64.shr_u
    local.get 0
    i64.const 43
    i64.shl
    i64.or)
  (func $i64_11 (param i64 i64) (result i64)
    local.get 0
    i64.const 53
    i64.shl
    local.get 1
    i64.const 11
    i64.shr_u
    i64.or)
)
;; wasm[0]::function[0]::i32_21:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       shldl   $0xb, %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::i32_21_swapped:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       shldl   $0xb, %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::i32_11:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       shldl   $0x15, %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]::i64_21:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       shldq   $0x2b, %rcx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]::i64_21_swapped:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       shldq   $0x2b, %rcx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]::i64_11:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       shldq   $0x35, %rcx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
