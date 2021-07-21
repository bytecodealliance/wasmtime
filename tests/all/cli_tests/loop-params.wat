  (module
    (func (export "run") (result i32)
      (local $i i32)
      (i32.const 0) ;; sum
      (i32.const 10) ;; i
      (loop $loop (param i32 i32) (result i32)
        (local.tee $i)
        (i32.add) ;; sum = i + sum
        (i32.sub (local.get $i) (i32.const 1))
        (i32.eqz (local.tee $i))
        (if (param i32) (result i32)
            (then)
            (else (local.get $i) (br $loop))))))
