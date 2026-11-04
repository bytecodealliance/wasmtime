;;! reference_types = true
;;! function_references = true
;;! gc = true

;; A function type with a supertype.
(component definition
  (import "m" (core module
    (type $super (sub (func)))
    (type $sub (sub $super (func)))
    (import "" "f" (func (type $sub))))))

;; A function type referring to a struct type in another rec group.
(component definition
  (import "m" (core module
    (type $s (struct))
    (type $f (func (param (ref $s))))
    (import "" "f" (func (type $f))))))

;; A rec group whose members refer to types in two other rec groups, one of
;; which itself depends on another rec group.
(component definition
  (import "m" (core module
    (type $a (struct))
    (type $b (sub (struct (field (ref $a)))))
    (rec
      (type $f (func (param (ref $b))))
      (type (sub $b (struct (field (ref $a))))))
    (import "" "f" (func (type $f))))))
