;;! target = "x86_64"
;;! test = "compile"
;;! filter = "wasm[1]"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

(component
  (component $A
    (core module $M
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42))
      )
    )

    (core instance $m (instantiate $M))

    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))

    (core func $f' (canon lower (func $f)))

    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1234))
      )
    )

    (core instance $n
      (instantiate $N
        (with "" (instance (export "f'" (func $f'))))
      )
    )

    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))
    )
  )

  (instance $a (instantiate $A))
  (instance $b
    (instantiate $B
      (with "f" (func $a "f"))
    )
  )

  (export "g" (func $b "g"))
)

;; wasm[1]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    0x48(%rdi), %rdi
;;       movq    0xa8(%rdi), %rdi
;;       movl    (%rdi), %edi
;;       testl   %edi, %edi
;;       je      0x43
;;   39: movl    $0x4fc, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   43: ud2
