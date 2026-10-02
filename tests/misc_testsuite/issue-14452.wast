;;! gc = true
;;! exceptions = true

(module
  (type $obj (struct))
  (type $garbage (array (mut i32)))
  (tag $exn)
  (global $keep_garbage (mut (ref null $garbage)) (ref.null $garbage))
  (global $root (mut (ref null $obj)) (ref.null $obj))

  (func $collect_then_throw
    i32.const 100000
    array.new_default $garbage
    global.set $keep_garbage
    i32.const 100000
    array.new_default $garbage
    global.set $keep_garbage
    i32.const 100000
    array.new_default $garbage
    global.set $keep_garbage
    throw $exn)

  (func (export "run") (local $local (ref null $obj))
    struct.new $obj
    local.set $local
    local.get $local
    global.set $root
    block $outer_handler
      try_table (catch $exn $outer_handler)
        block $skip_throw
          block $inner_handler
            try_table (catch $exn $inner_handler)
              br $skip_throw
            end
          end
          throw $exn
        end
        call $collect_then_throw
      end
      return
    end
    local.get $local
    global.get $root
    ref.eq
    i32.eqz
    if
      unreachable
    end)
)

(assert_return (invoke "run"))
