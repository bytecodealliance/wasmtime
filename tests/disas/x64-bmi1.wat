;;! target = "x86_64"
;;! test = "compile"
;;! flags = "-Ccranelift-has-bmi1 -Ccranelift-has-avx"

(module
  (func (export "blsi32") (param i32) (result i32)
    (i32.and
      (local.get 0)
      (i32.sub (i32.const 0) (local.get 0))))

  (func (export "blsi64") (param i64) (result i64)
    (i64.and
      (local.get 0)
      (i64.sub (i64.const 0) (local.get 0))))

  (func (export "blsr32") (param i32) (result i32)
    (i32.and
      (local.get 0)
      (i32.add (local.get 0) (i32.const -1))))

  (func (export "blsr64") (param i64) (result i64)
    (i64.and
      (local.get 0)
      (i64.sub (local.get 0) (i64.const 1))))

  (func (export "blsmsk32") (param i32) (result i32)
    (i32.xor
      (local.get 0)
      (i32.sub (local.get 0) (i32.const 1))))

  (func (export "blsmsk64") (param i64) (result i64)
    (i64.xor
      (local.get 0)
      (i64.add (local.get 0) (i64.const -1))))

  (func (export "tzcnt32") (param i32) (result i32)
    (i32.ctz (local.get 0)))

  (func (export "tzcnt64") (param i64) (result i64)
    (i64.ctz (local.get 0)))

  (func (export "andn32") (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.xor (local.get 1) (i32.const -1))))

  (func (export "andn64") (param i64 i64) (result i64)
    (i64.and (local.get 0) (i64.xor (local.get 1) (i64.const -1))))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       blsil   %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       blsiq   %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       blsrl   %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       blsrq   %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       blsmskl %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       blsmskq %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       tzcntl  %edx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       tzcntq  %rdx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[8]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       andnl   %edx, %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[9]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       andnq   %rdx, %rcx, %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
