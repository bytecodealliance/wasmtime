;;! gc = true

;; Empty core rec groups define no types.
(component (core rec))
(component (core rec) (core type (module)))
(component (core type (sub (array i8))) (core rec))
