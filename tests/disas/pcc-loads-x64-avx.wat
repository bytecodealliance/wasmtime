;;! target = "x86_64"
;;! test = "compile"
;;! flags = [ "-Oopt-level=2", "-Cpcc=y", "-Ccranelift-has-avx=true" ]

(module
  (memory 1 1)
  (func (export "load_f32") (param i32) (result f32)
    local.get 0
    f32.load)
  (func (export "load_f64") (param i32) (result f64)
    local.get 0
    f64.load)
  (func (export "store_f32") (param i32 f32)
    local.get 0
    local.get 1
    f32.store)
  (func (export "store_f64") (param i32 f64)
    local.get 0
    local.get 1
    f64.store))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x60(%rdi), %r9
;;       movl    %edx, %r10d
;;       vmovss  (%r9, %r10), %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x60(%rdi), %r9
;;       movl    %edx, %r10d
;;       vmovsd  (%r9, %r10), %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x60(%rdi), %r9
;;       movl    %edx, %r10d
;;       vmovss  %xmm0, (%r9, %r10)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x60(%rdi), %r9
;;       movl    %edx, %r10d
;;       vmovsd  %xmm0, (%r9, %r10)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
