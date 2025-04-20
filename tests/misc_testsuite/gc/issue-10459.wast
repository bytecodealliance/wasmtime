;;! gc = true

(module
  (type $func (func))

  (type $super (sub (struct (field (ref $func)))))
  (type $sub (sub final $super (struct (field (ref $func)) (field (ref eq)))))

  (elem declare func $f)
  (func $f)

  (func (export "run")
    (drop
      (struct.get $super 0
        (struct.new $sub
          (ref.func $f)
          (ref.i31 (i32.const 0)))))
  )
)

(assert_return (invoke "run"))
