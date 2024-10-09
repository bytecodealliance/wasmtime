;; Test that we properly handle empty rec groups and when we have more rec
;; groups defined in the type section than actual types (and therefore the
;; length that the type section reports is greater than the length of the types
;; index space).

(module
  (rec)
  (rec)
  (rec)
  (rec)
  (rec)
  (rec)
  (rec)
  (rec)
  (rec)
  (rec)
  (type (func (param i32) (result i32)))
)
