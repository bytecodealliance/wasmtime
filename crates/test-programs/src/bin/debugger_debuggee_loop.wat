(module
  (memory (export "memory") 1)
  (func (export "_start")
    (block $exit
      (loop $loop
        (br_if $exit (i32.load8_u (i32.const 0)))
        (br $loop)
      )
    )
  )
)
