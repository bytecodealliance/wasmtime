;;! target = "x86_64"
;;! test = 'compile'
;;! flags = '-Wcomponent-model-async -Wcomponent-model-threading -Cinlining'

(component
  (core func $context.get0 (canon context.get i32 0))
  (core func $context.get1 (canon context.get i32 1))
  (core func $context.set0 (canon context.set i32 0))
  (core func $context.set1 (canon context.set i32 1))

  (core module $m
    (import "" "get0" (func $get0 (result i32)))
    (import "" "get1" (func $get1 (result i32)))
    (import "" "set0" (func $set0 (param i32)))
    (import "" "set1" (func $set1 (param i32)))

    (func $g0 (export "get0") (result i32) (call $get0))
    (func $g1 (export "get1") (result i32) (call $get1))
    (func $s0 (export "set0") (param i32) (call $set0 (local.get 0)))
    (func $s1 (export "set1") (param i32) (call $set1 (local.get 0)))
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "get0" (func $context.get0))
      (export "get1" (func $context.get1))
      (export "set0" (func $context.set0))
      (export "set1" (func $context.set1))
    ))
  ))

  (func (export "get0") (result u32) (canon lift (core func $i "get0")))
  (func (export "get1") (result u32) (canon lift (core func $i "get1")))
  (func (export "set0") (param "x" u32) (canon lift (core func $i "set0")))
  (func (export "set1") (param "x" u32) (canon lift (core func $i "set1")))
)


;; wasm[0]::function[4]::g0:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rsi
;;       movl    0x80(%rsi), %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]::g1:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rsi
;;       movl    0x84(%rsi), %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]::s0:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rsi
;;       movl    %edx, 0x80(%rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]::s1:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rsi
;;       movl    %edx, 0x84(%rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
