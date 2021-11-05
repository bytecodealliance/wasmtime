(module (func (export "empty")))

(invoke "empty")

(module binary
  "\00asm\01\00\00\00"    ;; module header

  "\00"             ;; custom section id 0
  "\0e"             ;; section size
  "\04name"         ;; this is the `name` custom section
  "\01"             ;; function name subsection
  "\07"             ;; function name subsection size
  "\01"             ;; 1 function name mapping
  "\ff\ff\ff\ff\0f" ;; index == u32::MAX
  "\00"             ;; empty string name
)

(module binary
  "\00asm\01\00\00\00"    ;; module header

  "\00"             ;; custom section id 0
  "\10"             ;; section size
  "\04name"         ;; this is the `name` custom section
  "\02"             ;; local name subsection
  "\09"             ;; local name subsection size
  "\01"             ;; 1 indirect name map
  "\ff\ff\ff\ff\0f" ;; index == u32::MAX (function)
  "\01"             ;; 1 name mapping
  "\00"             ;; index == 0 (local)
  "\00"             ;; empty string name
)

(module binary
  "\00asm\01\00\00\00"    ;; module header

  "\00"             ;; custom section id 0
  "\10"             ;; section size
  "\04name"         ;; this is the `name` custom section
  "\02"             ;; local name subsection
  "\09"             ;; local name subsection size
  "\01"             ;; 1 indirect name map
  "\00"             ;; index == 0 (function)
  "\01"             ;; 1 name mapping
  "\ff\ff\ff\ff\0f" ;; index == u32::MAX (local)
  "\00"             ;; empty string name
)
