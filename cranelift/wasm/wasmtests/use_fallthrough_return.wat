(module
  (memory 1)
  (func $main (param i32)
      (if
          (get_local 0)
          (then (return))
          (else (unreachable))
       )
  )
)
