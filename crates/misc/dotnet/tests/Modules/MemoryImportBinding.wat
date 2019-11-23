(module
  (import "" "mem" (memory 1))
  (data (i32.const 0) "Hello World")
  (data (i32.const 20) "\01")
  (data (i32.const 21) "\02\00")
  (data (i32.const 23) "\03\00\00\00")
  (data (i32.const 27) "\04\00\00\00\00\00\00\00")
  (data (i32.const 35) "\00\00\a0\40")
  (data (i32.const 39) "\00\00\00\00\00\00\18\40")
  (data (i32.const 48) "\07\00\00\00\00\00\00\00")
  (func (export "ReadByte") (result i32)
    i32.const 20
    i32.load8_s
  )
  (func (export "ReadInt16") (result i32)
    i32.const 21
    i32.load16_s
  )
  (func (export "ReadInt32") (result i32)
    i32.const 23
    i32.load
  )
  (func (export "ReadInt64") (result i64)
    i32.const 27
    i64.load
  )
  (func (export "ReadFloat32") (result f32)
    i32.const 35
    f32.load
  )
  (func (export "ReadFloat64") (result f64)
    i32.const 39
    f64.load
  )
  (func (export "ReadIntPtr") (result i64)
    i32.const 48
    i64.load
  )
)
