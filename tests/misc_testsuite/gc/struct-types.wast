;;! gc = true

(module
  (type (struct))
  (type (struct (field)))
  (type (struct (field i8)))
  (type (struct (field i8 i8 i8 i8)))
  (type (struct (field $x1 i32) (field $y1 i32)))
  (type (struct (field i8 i16 i32 i64 f32 f64 anyref funcref (ref 0) (ref null 1))))
  (type (struct (field i32 i64 i8) (field) (field) (field (ref null i31) anyref structref arrayref)))
  (type (struct (field $x2 i32) (field f32 f64) (field $y2 i32)))
)
