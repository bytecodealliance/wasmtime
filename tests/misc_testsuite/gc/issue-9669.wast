;;! gc = true

(module
  (type $empty (struct))
  (type $thing (struct
    (field $field1 (ref $empty))
    (field $field2 (ref $empty))))

  (func (export "run")
    (local $object (ref $thing))

    struct.new $empty
    struct.new $empty
    struct.new $thing

    local.tee $object
    struct.get $thing $field1
    drop

    local.get $object
    struct.get $thing $field2
    drop))

(assert_return (invoke "run"))
