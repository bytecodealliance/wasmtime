(module
  (memory 1)
  (func $main (local i32)
      (local.set 0 (i32.sub (i32.const 4) (i32.const 4)))
      (if
          (local.get 0)
          (then unreachable)
          (else (drop (i32.mul (i32.const 6) (local.get 0))))
       )
  )
  (start $main)
  (data (i32.const 0) "abcdefgh")
)
