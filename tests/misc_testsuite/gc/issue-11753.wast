;;! gc = true
;;! exceptions = true

(module
  (type $s (struct (field (mut i32))))

  (import "wasmtime" "gc" (func $gc))

  (func $observe (param (ref null $s)) (result (ref null $s))
    local.get 0
  )

  (func (export "run") (result i32)
    (struct.new $s (i32.const 42))
    call $observe

    block $b
      try_table (catch_all $b)
        call $gc
        (drop (struct.new $s (i32.const 0)))
      end
    end

    struct.get $s 0
  )
)

(assert_return (invoke "run") (i32.const 42))
