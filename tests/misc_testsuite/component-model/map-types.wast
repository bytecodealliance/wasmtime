;;! component_model_map = true

(component
  (type (map u32 string))
  (type (map string u32))
  (type (map u32 u32))
  (type (map string (map u32 string)))
)

;; Every type in the `keytype` grammar can be used as a map key.
(component
  (type (map bool u32))
  (type (map s8 u32))
  (type (map u8 u32))
  (type (map s16 u32))
  (type (map u16 u32))
  (type (map s32 u32))
  (type (map u32 u32))
  (type (map s64 u32))
  (type (map u64 u32))
  (type (map char u32))
  (type (map string u32))
)

;; Any other type is rejected as a map key.
(assert_invalid
  (component (type (map f32 u32)))
  "invalid map key type")

(assert_invalid
  (component (type (map f64 u32)))
  "invalid map key type")

(assert_invalid
  (component (type (map (list u32) u32)))
  "invalid map key type")

(assert_invalid
  (component
    (type $r (record (field "a" u32)))
    (type (map $r u32)))
  "invalid map key type")

(assert_invalid
  (component (type (map (option u32) u32)))
  "invalid map key type")

(assert_invalid
  (component (type (map (tuple u32 u32) u32)))
  "invalid map key type")

(assert_invalid
  (component
    (type $t (resource (rep i32)))
    (type (map (own $t) u32)))
  "invalid map key type")

(assert_invalid
  (component
    (type $t (resource (rep i32)))
    (type (map (borrow $t) u32)))
  "invalid map key type")
