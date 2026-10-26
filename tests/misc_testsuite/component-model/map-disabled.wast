;;! component_model_map = false

(assert_invalid
  (component (type (map u32 string)))
  "Maps require the component model map feature")
