;;! gc = true
;;! simd = true

(module $X
  (rec
    (type (;0;) (array i8))
    (type (;1;) (sub (struct (field i16))))
    (type (;2;) (sub 1 (struct (field i16) (field i16) (field i16) (field i16))))
    (type (;3;) (func (result f32)))
    (type (;4;) (sub (struct (field i31ref) (field i16) (field (mut f32)) (field v128) (field i8) (field (mut i16)) (field externref) (field (mut i16)) (field i16) (field i16) (field i16) (field i8) (field (mut v128)) (field i8) (field (mut i32)) (field i8) (field (mut i8)) (field (mut i8)))))
    (type (;5;) (sub final 1 (struct (field i16) (field i8))))
    (type (;6;) (sub (array (mut v128))))
    (type (;7;) (sub (func (param f64 i32) (result i32 f32 externref externref externref (ref null 3) f64 f64))))
    (type (;8;) (sub (func (result i64 i64 f32 f32 v128 i32))))
    (type (;9;) (sub (struct (field (mut v128)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field i32))))
    (type (;10;) (sub (struct (field (mut i16)) (field (mut externref)) (field (mut f64)) (field (mut externref)) (field (mut i8)) (field (mut i8)) (field i8) (field i8) (field i16) (field (mut i8)) (field (mut i8)) (field f32) (field (mut i8)) (field (mut i8)) (field anyref) (field funcref) (field i16))))
    (type (;11;) (func (param externref (ref null 3)) (result f32)))
    (type (;12;) (struct (field i8) (field i16) (field (mut externref)) (field (mut externref)) (field (mut externref)) (field (mut externref)) (field (mut externref)) (field (mut nullfuncref)) (field (mut i8)) (field i8) (field i8) (field i16) (field (mut i8)) (field (mut i8)) (field i16) (field (mut externref)) (field (mut v128))))
    (type (;13;) (sub (struct (field f64) (field f32) (field (mut (ref null 3))) (field i8) (field i32) (field i16) (field nullfuncref) (field (mut structref)) (field i16) (field (mut i8)) (field (mut f64)) (field (mut i8)) (field i8) (field i8) (field i16) (field i8) (field i32))))
    (type (;14;) (func (result i64)))
    (type (;15;) (sub (array i8)))
    (type (;16;) (sub 2 (struct (field i16) (field i16) (field i16) (field i16))))
    (type (;17;) (sub (struct (field i16) (field i16) (field (mut i8)) (field (mut i8)) (field i8) (field i8) (field i8))))
    (type (;18;) (sub (array (mut i8))))
    (type (;19;) (sub (array (mut i8))))
    (type (;20;) (sub (array (mut i8))))
    (type (;21;) (struct (field i16) (field (mut i16)) (field funcref) (field (mut i16)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i32)) (field (mut i8)) (field (mut f64)) (field (mut i8)) (field nullexternref) (field (mut i16)) (field i16)))
    (type (;22;) (sub (func)))
    (type (;23;) (func))
    (type (;24;) (sub (struct (field i8) (field (mut i64)) (field i8) (field (mut i64)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)))))
    (type (;25;) (sub (array (mut i8))))
    (type (;26;) (func))
    (type (;27;) (sub 2 (struct (field i16) (field i16) (field i16) (field i16))))
    (type (;28;) (sub (func (param i32))))
    (type (;29;) (sub (array (mut i16))))
    (type (;30;) (sub (array (mut i8))))
    (type (;31;) (func))
    (type (;32;) (sub 1 (struct (field i16) (field i16) (field (mut i8)) (field (mut i8)) (field i8))))
    (type (;33;) (sub (func (param i32 i64))))
    (type (;34;) (sub final 2 (struct (field i16) (field i16) (field i16) (field i16))))
    (type (;35;) (struct (field i8) (field (ref null 1)) (field i8) (field i8) (field i16) (field (mut i8))))
    (type (;36;) (struct (field i16) (field i16) (field i16)))
    (type (;37;) (func (param f32) (result i32 (ref null 1))))
    (type (;38;) (sub 1 (struct (field i16) (field i8) (field i8) (field (mut i16)))))
    (type (;39;) (sub 1 (struct (field i16) (field (mut i8)))))
    (type (;40;) (sub (array (mut i16))))
    (type (;41;) (sub (array (mut i8))))
  )
  (func (export "") (type 8) (result i64 i64 f32 f32 v128 i32)
    i64.const 0
    i64.const 0
    f32.const 0.0
    f32.const 0.0
    v128.const i64x2 0 0
    i32.const 0
  )
)

(module
  (rec
    (type (;0;) (array i8))
    (type (;1;) (sub (struct (field i16))))
    (type (;2;) (sub 1 (struct (field i16) (field i16) (field i16) (field i16))))
    (type (;3;) (func (result f32)))
    (type (;4;) (sub (struct (field i31ref) (field i16) (field (mut f32)) (field v128) (field i8) (field (mut i16)) (field externref) (field (mut i16)) (field i16) (field i16) (field i16) (field i8) (field (mut v128)) (field i8) (field (mut i32)) (field i8) (field (mut i8)) (field (mut i8)))))
    (type (;5;) (sub final 1 (struct (field i16) (field i8))))
    (type (;6;) (sub (array (mut v128))))
    (type (;7;) (sub (func (param f64 i32) (result i32 f32 externref externref externref (ref null 3) f64 f64))))
    (type (;8;) (sub (func (result i64 i64 f32 f32 v128 i32))))
    (type (;9;) (sub (struct (field (mut v128)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field i32))))
    (type (;10;) (sub (struct (field (mut i16)) (field (mut externref)) (field (mut f64)) (field (mut externref)) (field (mut i8)) (field (mut i8)) (field i8) (field i8) (field i16) (field (mut i8)) (field (mut i8)) (field f32) (field (mut i8)) (field (mut i8)) (field anyref) (field funcref) (field i16))))
    (type (;11;) (func (param externref (ref null 3)) (result f32)))
    (type (;12;) (struct (field i8) (field i16) (field (mut externref)) (field (mut externref)) (field (mut externref)) (field (mut externref)) (field (mut externref)) (field (mut nullfuncref)) (field (mut i8)) (field i8) (field i8) (field i16) (field (mut i8)) (field (mut i8)) (field i16) (field (mut externref)) (field (mut v128))))
    (type (;13;) (sub (struct (field f64) (field f32) (field (mut (ref null 3))) (field i8) (field i32) (field i16) (field nullfuncref) (field (mut structref)) (field i16) (field (mut i8)) (field (mut f64)) (field (mut i8)) (field i8) (field i8) (field i16) (field i8) (field i32))))
    (type (;14;) (func (result i64)))
    (type (;15;) (sub (array i8)))
    (type (;16;) (sub 2 (struct (field i16) (field i16) (field i16) (field i16))))
    (type (;17;) (sub (struct (field i16) (field i16) (field (mut i8)) (field (mut i8)) (field i8) (field i8) (field i8))))
    (type (;18;) (sub (array (mut i8))))
    (type (;19;) (sub (array (mut i8))))
    (type (;20;) (sub (array (mut i8))))
    (type (;21;) (struct (field i16) (field (mut i16)) (field funcref) (field (mut i16)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i32)) (field (mut i8)) (field (mut f64)) (field (mut i8)) (field nullexternref) (field (mut i16)) (field i16)))
    (type (;22;) (sub (func)))
    (type (;23;) (func))
    (type (;24;) (sub (struct (field i8) (field (mut i64)) (field i8) (field (mut i64)) (field (mut i8)) (field (mut i8)) (field (mut i8)) (field (mut i8)))))
    (type (;25;) (sub (array (mut i8))))
    (type (;26;) (func))
    (type (;27;) (sub 2 (struct (field i16) (field i16) (field i16) (field i16))))
    (type (;28;) (sub (func (param i32))))
    (type (;29;) (sub (array (mut i16))))
    (type (;30;) (sub (array (mut i8))))
    (type (;31;) (func))
    (type (;32;) (sub 1 (struct (field i16) (field i16) (field (mut i8)) (field (mut i8)) (field i8))))
    (type (;33;) (sub (func (param i32 i64))))
    (type (;34;) (sub final 2 (struct (field i16) (field i16) (field i16) (field i16))))
    (type (;35;) (struct (field i8) (field (ref null 1)) (field i8) (field i8) (field i16) (field (mut i8))))
    (type (;36;) (struct (field i16) (field i16) (field i16)))
    (type (;37;) (func (param f32) (result i32 (ref null 1))))
    (type (;38;) (sub 1 (struct (field i16) (field i8) (field i8) (field (mut i16)))))
    (type (;39;) (sub 1 (struct (field i16) (field (mut i8)))))
    (type (;40;) (sub (array (mut i16))))
    (type (;41;) (sub (array (mut i8))))
  )
  (rec)
  (rec)
  (rec
    (type (;42;) (struct (field i8) (field (ref null 1)) (field i8) (field i16) (field (mut i16)) (field (mut i8))))
    (type (;43;) (sub (array (mut i8))))
    (type (;44;) (sub 1 (struct (field i16))))
    (type (;45;) (sub (array i8)))
    (type (;46;) (array i16))
    (type (;47;) (sub (func (result v128 f32))))
    (type (;48;) (sub (struct (field (mut i8)) (field i16) (field (mut i8)) (field i8) (field i8) (field (mut i16)) (field i8) (field (mut i8)))))
    (type (;49;) (sub final 38 (struct (field i16) (field i8) (field i8) (field (mut i16)) (field i16) (field f32) (field i16) (field i16))))
    (type (;50;) (sub (func)))
    (type (;51;) (sub 43 (array (mut i8))))
  )
  (import "X" "" (func (;0;) (type 8)))
  (global (;0;) (mut v128) v128.const i32x4 0x00000000 0x00000000 0x00000000 0x00000000)
  (export "" (func 1))
  (export "\u{b}|" (global 0))
  (func (;1;) (type 28) (param i32)
    (local i64)
    struct.new_default 48
    v128.const i32x4 0x0b081044 0x48408020 0x04642040 0x02024204
    array.new_fixed 6 0
    struct.new_default 49
    struct.new_default 13
    struct.new_default 21
    extern.convert_any
    any.convert_extern
    v128.const i32x4 0x81a04120 0x83701024 0x04a14520 0xd0108790
    struct.new_default 4
    v128.const i32x4 0x04048805 0x2a204020 0x3f102010 0x18bffeff
    drop
    struct.new_default 1
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    global.get 0
    v128.xor
    global.set 0
    drop
  )
)

(invoke "" (i32.const 0))
