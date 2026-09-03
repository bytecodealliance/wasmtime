;;! target = "x86_64"

;; Wasmtime hands every defined, non-exported global its own alias region, so
;; the function below asks for 257 global regions on top of the handful of `VM*`
;; regions its prologue needs. Alias regions are hashed down into a single byte,
;; however, so we emit at most 256 of them no matter how many are asked for: the
;; region table in the expectation below is well short of 259 entries, and
;; several globals share a region as a result.

(module
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0)) (global (mut i32) (i32.const 0))
  (global (mut i32) (i32.const 0))

  (func (export "sum") (result i32)
    global.get 0 global.get 1 i32.add global.get 2 i32.add global.get 3
    i32.add global.get 4 i32.add global.get 5 i32.add global.get 6
    i32.add global.get 7 i32.add global.get 8 i32.add global.get 9
    i32.add global.get 10 i32.add global.get 11 i32.add global.get 12
    i32.add global.get 13 i32.add global.get 14 i32.add global.get 15
    i32.add global.get 16 i32.add global.get 17 i32.add global.get 18
    i32.add global.get 19 i32.add global.get 20 i32.add global.get 21
    i32.add global.get 22 i32.add global.get 23 i32.add global.get 24
    i32.add global.get 25 i32.add global.get 26 i32.add global.get 27
    i32.add global.get 28 i32.add global.get 29 i32.add global.get 30
    i32.add global.get 31 i32.add global.get 32 i32.add global.get 33
    i32.add global.get 34 i32.add global.get 35 i32.add global.get 36
    i32.add global.get 37 i32.add global.get 38 i32.add global.get 39
    i32.add global.get 40 i32.add global.get 41 i32.add global.get 42
    i32.add global.get 43 i32.add global.get 44 i32.add global.get 45
    i32.add global.get 46 i32.add global.get 47 i32.add global.get 48
    i32.add global.get 49 i32.add global.get 50 i32.add global.get 51
    i32.add global.get 52 i32.add global.get 53 i32.add global.get 54
    i32.add global.get 55 i32.add global.get 56 i32.add global.get 57
    i32.add global.get 58 i32.add global.get 59 i32.add global.get 60
    i32.add global.get 61 i32.add global.get 62 i32.add global.get 63
    i32.add global.get 64 i32.add global.get 65 i32.add global.get 66
    i32.add global.get 67 i32.add global.get 68 i32.add global.get 69
    i32.add global.get 70 i32.add global.get 71 i32.add global.get 72
    i32.add global.get 73 i32.add global.get 74 i32.add global.get 75
    i32.add global.get 76 i32.add global.get 77 i32.add global.get 78
    i32.add global.get 79 i32.add global.get 80 i32.add global.get 81
    i32.add global.get 82 i32.add global.get 83 i32.add global.get 84
    i32.add global.get 85 i32.add global.get 86 i32.add global.get 87
    i32.add global.get 88 i32.add global.get 89 i32.add global.get 90
    i32.add global.get 91 i32.add global.get 92 i32.add global.get 93
    i32.add global.get 94 i32.add global.get 95 i32.add global.get 96
    i32.add global.get 97 i32.add global.get 98 i32.add global.get 99
    i32.add global.get 100 i32.add global.get 101 i32.add global.get 102
    i32.add global.get 103 i32.add global.get 104 i32.add global.get 105
    i32.add global.get 106 i32.add global.get 107 i32.add global.get 108
    i32.add global.get 109 i32.add global.get 110 i32.add global.get 111
    i32.add global.get 112 i32.add global.get 113 i32.add global.get 114
    i32.add global.get 115 i32.add global.get 116 i32.add global.get 117
    i32.add global.get 118 i32.add global.get 119 i32.add global.get 120
    i32.add global.get 121 i32.add global.get 122 i32.add global.get 123
    i32.add global.get 124 i32.add global.get 125 i32.add global.get 126
    i32.add global.get 127 i32.add global.get 128 i32.add global.get 129
    i32.add global.get 130 i32.add global.get 131 i32.add global.get 132
    i32.add global.get 133 i32.add global.get 134 i32.add global.get 135
    i32.add global.get 136 i32.add global.get 137 i32.add global.get 138
    i32.add global.get 139 i32.add global.get 140 i32.add global.get 141
    i32.add global.get 142 i32.add global.get 143 i32.add global.get 144
    i32.add global.get 145 i32.add global.get 146 i32.add global.get 147
    i32.add global.get 148 i32.add global.get 149 i32.add global.get 150
    i32.add global.get 151 i32.add global.get 152 i32.add global.get 153
    i32.add global.get 154 i32.add global.get 155 i32.add global.get 156
    i32.add global.get 157 i32.add global.get 158 i32.add global.get 159
    i32.add global.get 160 i32.add global.get 161 i32.add global.get 162
    i32.add global.get 163 i32.add global.get 164 i32.add global.get 165
    i32.add global.get 166 i32.add global.get 167 i32.add global.get 168
    i32.add global.get 169 i32.add global.get 170 i32.add global.get 171
    i32.add global.get 172 i32.add global.get 173 i32.add global.get 174
    i32.add global.get 175 i32.add global.get 176 i32.add global.get 177
    i32.add global.get 178 i32.add global.get 179 i32.add global.get 180
    i32.add global.get 181 i32.add global.get 182 i32.add global.get 183
    i32.add global.get 184 i32.add global.get 185 i32.add global.get 186
    i32.add global.get 187 i32.add global.get 188 i32.add global.get 189
    i32.add global.get 190 i32.add global.get 191 i32.add global.get 192
    i32.add global.get 193 i32.add global.get 194 i32.add global.get 195
    i32.add global.get 196 i32.add global.get 197 i32.add global.get 198
    i32.add global.get 199 i32.add global.get 200 i32.add global.get 201
    i32.add global.get 202 i32.add global.get 203 i32.add global.get 204
    i32.add global.get 205 i32.add global.get 206 i32.add global.get 207
    i32.add global.get 208 i32.add global.get 209 i32.add global.get 210
    i32.add global.get 211 i32.add global.get 212 i32.add global.get 213
    i32.add global.get 214 i32.add global.get 215 i32.add global.get 216
    i32.add global.get 217 i32.add global.get 218 i32.add global.get 219
    i32.add global.get 220 i32.add global.get 221 i32.add global.get 222
    i32.add global.get 223 i32.add global.get 224 i32.add global.get 225
    i32.add global.get 226 i32.add global.get 227 i32.add global.get 228
    i32.add global.get 229 i32.add global.get 230 i32.add global.get 231
    i32.add global.get 232 i32.add global.get 233 i32.add global.get 234
    i32.add global.get 235 i32.add global.get 236 i32.add global.get 237
    i32.add global.get 238 i32.add global.get 239 i32.add global.get 240
    i32.add global.get 241 i32.add global.get 242 i32.add global.get 243
    i32.add global.get 244 i32.add global.get 245 i32.add global.get 246
    i32.add global.get 247 i32.add global.get 248 i32.add global.get 249
    i32.add global.get 250 i32.add global.get 251 i32.add global.get 252
    i32.add global.get 253 i32.add global.get 254 i32.add global.get 255
    i32.add global.get 256 i32.add
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 250 ""
;;     region3 = 135 ""
;;     region4 = 76 ""
;;     region5 = 208 ""
;;     region6 = 152 ""
;;     region7 = 52 ""
;;     region8 = 113 ""
;;     region9 = 185 ""
;;     region10 = 251 ""
;;     region11 = 60 ""
;;     region12 = 100 ""
;;     region13 = 167 ""
;;     region14 = 71 ""
;;     region15 = 197 ""
;;     region16 = 236 ""
;;     region17 = 155 ""
;;     region18 = 124 ""
;;     region19 = 2 ""
;;     region20 = 242 ""
;;     region21 = 195 ""
;;     region22 = 212 ""
;;     region23 = 117 ""
;;     region24 = 194 ""
;;     region25 = 57 ""
;;     region26 = 47 ""
;;     region27 = 138 ""
;;     region28 = 129 ""
;;     region29 = 29 ""
;;     region30 = 174 ""
;;     region31 = 112 ""
;;     region32 = 15 ""
;;     region33 = 150 ""
;;     region34 = 36 ""
;;     region35 = 133 ""
;;     region36 = 63 ""
;;     region37 = 175 ""
;;     region38 = 40 ""
;;     region39 = 222 ""
;;     region40 = 178 ""
;;     region41 = 240 ""
;;     region42 = 187 ""
;;     region43 = 196 ""
;;     region44 = 81 ""
;;     region45 = 99 ""
;;     region46 = 192 ""
;;     region47 = 20 ""
;;     region48 = 118 ""
;;     region49 = 58 ""
;;     region50 = 249 ""
;;     region51 = 98 ""
;;     region52 = 173 ""
;;     region53 = 86 ""
;;     region54 = 110 ""
;;     region55 = 17 ""
;;     region56 = 35 ""
;;     region57 = 127 ""
;;     region58 = 146 ""
;;     region59 = 38 ""
;;     region60 = 128 ""
;;     region61 = 37 ""
;;     region62 = 50 ""
;;     region63 = 39 ""
;;     region64 = 3 ""
;;     region65 = 169 ""
;;     region66 = 31 ""
;;     region67 = 27 ""
;;     region68 = 156 ""
;;     region69 = 121 ""
;;     region70 = 69 ""
;;     region71 = 14 ""
;;     region72 = 79 ""
;;     region73 = 171 ""
;;     region74 = 48 ""
;;     region75 = 219 ""
;;     region76 = 8 ""
;;     region77 = 126 ""
;;     region78 = 77 ""
;;     region79 = 149 ""
;;     region80 = 209 ""
;;     region81 = 188 ""
;;     region82 = 46 ""
;;     region83 = 28 ""
;;     region84 = 244 ""
;;     region85 = 241 ""
;;     region86 = 87 ""
;;     region87 = 25 ""
;;     region88 = 137 ""
;;     region89 = 225 ""
;;     region90 = 193 ""
;;     region91 = 12 ""
;;     region92 = 216 ""
;;     region93 = 11 ""
;;     region94 = 181 ""
;;     region95 = 148 ""
;;     region96 = 213 ""
;;     region97 = 239 ""
;;     region98 = 42 ""
;;     region99 = 154 ""
;;     region100 = 254 ""
;;     region101 = 218 ""
;;     region102 = 78 ""
;;     region103 = 145 ""
;;     region104 = 246 ""
;;     region105 = 88 ""
;;     region106 = 217 ""
;;     region107 = 54 ""
;;     region108 = 162 ""
;;     region109 = 102 ""
;;     region110 = 108 ""
;;     region111 = 142 ""
;;     region112 = 115 ""
;;     region113 = 5 ""
;;     region114 = 157 ""
;;     region115 = 139 ""
;;     region116 = 64 ""
;;     region117 = 55 ""
;;     region118 = 105 ""
;;     region119 = 94 ""
;;     region120 = 70 ""
;;     region121 = 101 ""
;;     region122 = 73 ""
;;     region123 = 75 ""
;;     region124 = 59 ""
;;     region125 = 7 ""
;;     region126 = 179 ""
;;     region127 = 199 ""
;;     region128 = 131 ""
;;     region129 = 30 ""
;;     region130 = 253 ""
;;     region131 = 143 ""
;;     region132 = 97 ""
;;     region133 = 90 ""
;;     region134 = 166 ""
;;     region135 = 24 ""
;;     region136 = 205 ""
;;     region137 = 234 ""
;;     region138 = 61 ""
;;     region139 = 233 ""
;;     region140 = 116 ""
;;     region141 = 132 ""
;;     region142 = 229 ""
;;     region143 = 227 ""
;;     region144 = 238 ""
;;     region145 = 18 ""
;;     region146 = 210 ""
;;     region147 = 147 ""
;;     region148 = 159 ""
;;     region149 = 191 ""
;;     region150 = 214 ""
;;     region151 = 165 ""
;;     region152 = 224 ""
;;     region153 = 45 ""
;;     region154 = 177 ""
;;     region155 = 215 ""
;;     region156 = 109 ""
;;     region157 = 92 ""
;;     region158 = 22 ""
;;     region159 = 220 ""
;;     region160 = 93 ""
;;     region161 = 74 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @052d                               v2 = load.i32 notrap aligned region2 v0+48
;; @052f                               v3 = load.i32 notrap aligned region3 v0+64
;; @0531                               v4 = iadd v2, v3
;; @0532                               v5 = load.i32 notrap aligned region4 v0+80
;; @0534                               v6 = iadd v4, v5
;; @0535                               v7 = load.i32 notrap aligned region5 v0+96
;; @0537                               v8 = iadd v6, v7
;; @0538                               v9 = load.i32 notrap aligned region6 v0+112
;; @053a                               v10 = iadd v8, v9
;; @053b                               v11 = load.i32 notrap aligned region7 v0+128
;; @053d                               v12 = iadd v10, v11
;; @053e                               v13 = load.i32 notrap aligned region8 v0+144
;; @0540                               v14 = iadd v12, v13
;; @0541                               v15 = load.i32 notrap aligned region9 v0+160
;; @0543                               v16 = iadd v14, v15
;; @0544                               v17 = load.i32 notrap aligned region10 v0+176
;; @0546                               v18 = iadd v16, v17
;; @0547                               v19 = load.i32 notrap aligned region11 v0+192
;; @0549                               v20 = iadd v18, v19
;; @054a                               v21 = load.i32 notrap aligned region12 v0+208
;; @054c                               v22 = iadd v20, v21
;; @054d                               v23 = load.i32 notrap aligned region13 v0+224
;; @054f                               v24 = iadd v22, v23
;; @0550                               v25 = load.i32 notrap aligned region14 v0+240
;; @0552                               v26 = iadd v24, v25
;; @0553                               v27 = load.i32 notrap aligned region15 v0+256
;; @0555                               v28 = iadd v26, v27
;; @0556                               v29 = load.i32 notrap aligned region16 v0+272
;; @0558                               v30 = iadd v28, v29
;; @0559                               v31 = load.i32 notrap aligned region17 v0+288
;; @055b                               v32 = iadd v30, v31
;; @055c                               v33 = load.i32 notrap aligned region18 v0+304
;; @055e                               v34 = iadd v32, v33
;; @055f                               v35 = load.i32 notrap aligned region19 v0+320
;; @0561                               v36 = iadd v34, v35
;; @0562                               v37 = load.i32 notrap aligned region20 v0+336
;; @0564                               v38 = iadd v36, v37
;; @0565                               v39 = load.i32 notrap aligned region21 v0+352
;; @0567                               v40 = iadd v38, v39
;; @0568                               v41 = load.i32 notrap aligned region22 v0+368
;; @056a                               v42 = iadd v40, v41
;; @056b                               v43 = load.i32 notrap aligned region23 v0+384
;; @056d                               v44 = iadd v42, v43
;; @056e                               v45 = load.i32 notrap aligned region24 v0+400
;; @0570                               v46 = iadd v44, v45
;; @0571                               v47 = load.i32 notrap aligned region25 v0+416
;; @0573                               v48 = iadd v46, v47
;; @0574                               v49 = load.i32 notrap aligned region14 v0+432
;; @0576                               v50 = iadd v48, v49
;; @0577                               v51 = load.i32 notrap aligned region26 v0+448
;; @0579                               v52 = iadd v50, v51
;; @057a                               v53 = load.i32 notrap aligned region27 v0+464
;; @057c                               v54 = iadd v52, v53
;; @057d                               v55 = load.i32 notrap aligned region28 v0+480
;; @057f                               v56 = iadd v54, v55
;; @0580                               v57 = load.i32 notrap aligned region29 v0+496
;; @0582                               v58 = iadd v56, v57
;; @0583                               v59 = load.i32 notrap aligned region30 v0+512
;; @0585                               v60 = iadd v58, v59
;; @0586                               v61 = load.i32 notrap aligned region31 v0+528
;; @0588                               v62 = iadd v60, v61
;; @0589                               v63 = load.i32 notrap aligned region32 v0+544
;; @058b                               v64 = iadd v62, v63
;; @058c                               v65 = load.i32 notrap aligned region33 v0+560
;; @058e                               v66 = iadd v64, v65
;; @058f                               v67 = load.i32 notrap aligned region19 v0+576
;; @0591                               v68 = iadd v66, v67
;; @0592                               v69 = load.i32 notrap aligned region34 v0+592
;; @0594                               v70 = iadd v68, v69
;; @0595                               v71 = load.i32 notrap aligned region35 v0+608
;; @0597                               v72 = iadd v70, v71
;; @0598                               v73 = load.i32 notrap aligned region36 v0+624
;; @059a                               v74 = iadd v72, v73
;; @059b                               v75 = load.i32 notrap aligned region37 v0+640
;; @059d                               v76 = iadd v74, v75
;; @059e                               v77 = load.i32 notrap aligned region38 v0+656
;; @05a0                               v78 = iadd v76, v77
;; @05a1                               v79 = load.i32 notrap aligned region39 v0+672
;; @05a3                               v80 = iadd v78, v79
;; @05a4                               v81 = load.i32 notrap aligned region40 v0+688
;; @05a6                               v82 = iadd v80, v81
;; @05a7                               v83 = load.i32 notrap aligned region41 v0+704
;; @05a9                               v84 = iadd v82, v83
;; @05aa                               v85 = load.i32 notrap aligned region42 v0+720
;; @05ac                               v86 = iadd v84, v85
;; @05ad                               v87 = load.i32 notrap aligned region43 v0+736
;; @05af                               v88 = iadd v86, v87
;; @05b0                               v89 = load.i32 notrap aligned region44 v0+752
;; @05b2                               v90 = iadd v88, v89
;; @05b3                               v91 = load.i32 notrap aligned region45 v0+768
;; @05b5                               v92 = iadd v90, v91
;; @05b6                               v93 = load.i32 notrap aligned region46 v0+784
;; @05b8                               v94 = iadd v92, v93
;; @05b9                               v95 = load.i32 notrap aligned region27 v0+800
;; @05bb                               v96 = iadd v94, v95
;; @05bc                               v97 = load.i32 notrap aligned region44 v0+816
;; @05be                               v98 = iadd v96, v97
;; @05bf                               v99 = load.i32 notrap aligned region47 v0+832
;; @05c1                               v100 = iadd v98, v99
;; @05c2                               v101 = load.i32 notrap aligned region48 v0+848
;; @05c4                               v102 = iadd v100, v101
;; @05c5                               v103 = load.i32 notrap aligned region49 v0+864
;; @05c7                               v104 = iadd v102, v103
;; @05c8                               v105 = load.i32 notrap aligned region39 v0+880
;; @05ca                               v106 = iadd v104, v105
;; @05cb                               v107 = load.i32 notrap aligned region50 v0+896
;; @05cd                               v108 = iadd v106, v107
;; @05ce                               v109 = load.i32 notrap aligned region51 v0+912
;; @05d0                               v110 = iadd v108, v109
;; @05d1                               v111 = load.i32 notrap aligned region52 v0+928
;; @05d3                               v112 = iadd v110, v111
;; @05d4                               v113 = load.i32 notrap aligned region53 v0+944
;; @05d6                               v114 = iadd v112, v113
;; @05d7                               v115 = load.i32 notrap aligned region54 v0+960
;; @05d9                               v116 = iadd v114, v115
;; @05da                               v117 = load.i32 notrap aligned region55 v0+976
;; @05dc                               v118 = iadd v116, v117
;; @05dd                               v119 = load.i32 notrap aligned region56 v0+992
;; @05df                               v120 = iadd v118, v119
;; @05e0                               v121 = load.i32 notrap aligned region57 v0+1008
;; @05e2                               v122 = iadd v120, v121
;; @05e3                               v123 = load.i32 notrap aligned region45 v0+1024
;; @05e5                               v124 = iadd v122, v123
;; @05e6                               v125 = load.i32 notrap aligned region5 v0+1040
;; @05e8                               v126 = iadd v124, v125
;; @05e9                               v127 = load.i32 notrap aligned region28 v0+1056
;; @05eb                               v128 = iadd v126, v127
;; @05ec                               v129 = load.i32 notrap aligned region58 v0+1072
;; @05ee                               v130 = iadd v128, v129
;; @05ef                               v131 = load.i32 notrap aligned region59 v0+1088
;; @05f1                               v132 = iadd v130, v131
;; @05f2                               v133 = load.i32 notrap aligned region60 v0+1104
;; @05f4                               v134 = iadd v132, v133
;; @05f5                               v135 = load.i32 notrap aligned region61 v0+1120
;; @05f7                               v136 = iadd v134, v135
;; @05f8                               v137 = load.i32 notrap aligned region62 v0+1136
;; @05fa                               v138 = iadd v136, v137
;; @05fb                               v139 = load.i32 notrap aligned region63 v0+1152
;; @05fd                               v140 = iadd v138, v139
;; @05fe                               v141 = load.i32 notrap aligned region64 v0+1168
;; @0600                               v142 = iadd v140, v141
;; @0601                               v143 = load.i32 notrap aligned region43 v0+1184
;; @0603                               v144 = iadd v142, v143
;; @0604                               v145 = load.i32 notrap aligned region65 v0+1200
;; @0606                               v146 = iadd v144, v145
;; @0607                               v147 = load.i32 notrap aligned region66 v0+1216
;; @0609                               v148 = iadd v146, v147
;; @060a                               v149 = load.i32 notrap aligned region67 v0+1232
;; @060c                               v150 = iadd v148, v149
;; @060d                               v151 = load.i32 notrap aligned region46 v0+1248
;; @060f                               v152 = iadd v150, v151
;; @0610                               v153 = load.i32 notrap aligned region68 v0+1264
;; @0612                               v154 = iadd v152, v153
;; @0613                               v155 = load.i32 notrap aligned region34 v0+1280
;; @0615                               v156 = iadd v154, v155
;; @0616                               v157 = load.i32 notrap aligned region57 v0+1296
;; @0618                               v158 = iadd v156, v157
;; @0619                               v159 = load.i32 notrap aligned region69 v0+1312
;; @061b                               v160 = iadd v158, v159
;; @061c                               v161 = load.i32 notrap aligned region70 v0+1328
;; @061e                               v162 = iadd v160, v161
;; @061f                               v163 = load.i32 notrap aligned region71 v0+1344
;; @0621                               v164 = iadd v162, v163
;; @0622                               v165 = load.i32 notrap aligned region72 v0+1360
;; @0624                               v166 = iadd v164, v165
;; @0625                               v167 = load.i32 notrap aligned region16 v0+1376
;; @0627                               v168 = iadd v166, v167
;; @0628                               v169 = load.i32 notrap aligned region73 v0+1392
;; @062a                               v170 = iadd v168, v169
;; @062b                               v171 = load.i32 notrap aligned region74 v0+1408
;; @062d                               v172 = iadd v170, v171
;; @062e                               v173 = load.i32 notrap aligned region53 v0+1424
;; @0630                               v174 = iadd v172, v173
;; @0631                               v175 = load.i32 notrap aligned region75 v0+1440
;; @0633                               v176 = iadd v174, v175
;; @0634                               v177 = load.i32 notrap aligned region76 v0+1456
;; @0636                               v178 = iadd v176, v177
;; @0637                               v179 = load.i32 notrap aligned region77 v0+1472
;; @0639                               v180 = iadd v178, v179
;; @063a                               v181 = load.i32 notrap aligned region35 v0+1488
;; @063c                               v182 = iadd v180, v181
;; @063d                               v183 = load.i32 notrap aligned region28 v0+1504
;; @063f                               v184 = iadd v182, v183
;; @0640                               v185 = load.i32 notrap aligned region78 v0+1520
;; @0642                               v186 = iadd v184, v185
;; @0643                               v187 = load.i32 notrap aligned region79 v0+1536
;; @0645                               v188 = iadd v186, v187
;; @0646                               v189 = load.i32 notrap aligned region80 v0+1552
;; @0648                               v190 = iadd v188, v189
;; @0649                               v191 = load.i32 notrap aligned region81 v0+1568
;; @064b                               v192 = iadd v190, v191
;; @064c                               v193 = load.i32 notrap aligned region72 v0+1584
;; @064e                               v194 = iadd v192, v193
;; @064f                               v195 = load.i32 notrap aligned region27 v0+1600
;; @0651                               v196 = iadd v194, v195
;; @0652                               v197 = load.i32 notrap aligned region82 v0+1616
;; @0654                               v198 = iadd v196, v197
;; @0655                               v199 = load.i32 notrap aligned region83 v0+1632
;; @0657                               v200 = iadd v198, v199
;; @0658                               v201 = load.i32 notrap aligned region84 v0+1648
;; @065a                               v202 = iadd v200, v201
;; @065b                               v203 = load.i32 notrap aligned region85 v0+1664
;; @065d                               v204 = iadd v202, v203
;; @065e                               v205 = load.i32 notrap aligned region86 v0+1680
;; @0660                               v206 = iadd v204, v205
;; @0661                               v207 = load.i32 notrap aligned region77 v0+1696
;; @0663                               v208 = iadd v206, v207
;; @0664                               v209 = load.i32 notrap aligned region87 v0+1712
;; @0666                               v210 = iadd v208, v209
;; @0667                               v211 = load.i32 notrap aligned region88 v0+1728
;; @0669                               v212 = iadd v210, v211
;; @066a                               v213 = load.i32 notrap aligned region15 v0+1744
;; @066c                               v214 = iadd v212, v213
;; @066d                               v215 = load.i32 notrap aligned region89 v0+1760
;; @066f                               v216 = iadd v214, v215
;; @0670                               v217 = load.i32 notrap aligned region34 v0+1776
;; @0672                               v218 = iadd v216, v217
;; @0673                               v219 = load.i32 notrap aligned region90 v0+1792
;; @0675                               v220 = iadd v218, v219
;; @0676                               v221 = load.i32 notrap aligned region91 v0+1808
;; @0678                               v222 = iadd v220, v221
;; @0679                               v223 = load.i32 notrap aligned region88 v0+1824
;; @067b                               v224 = iadd v222, v223
;; @067c                               v225 = load.i32 notrap aligned region92 v0+1840
;; @067e                               v226 = iadd v224, v225
;; @067f                               v227 = load.i32 notrap aligned region53 v0+1856
;; @0681                               v228 = iadd v226, v227
;; @0682                               v229 = load.i32 notrap aligned region72 v0+1872
;; @0684                               v230 = iadd v228, v229
;; @0685                               v231 = load.i32 notrap aligned region69 v0+1888
;; @0687                               v232 = iadd v230, v231
;; @0688                               v233 = load.i32 notrap aligned region93 v0+1904
;; @068a                               v234 = iadd v232, v233
;; @068b                               v235 = load.i32 notrap aligned region77 v0+1920
;; @068d                               v236 = iadd v234, v235
;; @068e                               v237 = load.i32 notrap aligned region41 v0+1936
;; @0690                               v238 = iadd v236, v237
;; @0691                               v239 = load.i32 notrap aligned region61 v0+1952
;; @0693                               v240 = iadd v238, v239
;; @0694                               v241 = load.i32 notrap aligned region94 v0+1968
;; @0696                               v242 = iadd v240, v241
;; @0697                               v243 = load.i32 notrap aligned region95 v0+1984
;; @0699                               v244 = iadd v242, v243
;; @069a                               v245 = load.i32 notrap aligned region96 v0+2000
;; @069c                               v246 = iadd v244, v245
;; @069d                               v247 = load.i32 notrap aligned region97 v0+2016
;; @069f                               v248 = iadd v246, v247
;; @06a0                               v249 = load.i32 notrap aligned region41 v0+2032
;; @06a2                               v250 = iadd v248, v249
;; @06a3                               v251 = load.i32 notrap aligned region98 v0+2048
;; @06a5                               v252 = iadd v250, v251
;; @06a6                               v253 = load.i32 notrap aligned region11 v0+2064
;; @06a8                               v254 = iadd v252, v253
;; @06a9                               v255 = load.i32 notrap aligned region99 v0+2080
;; @06ab                               v256 = iadd v254, v255
;; @06ac                               v257 = load.i32 notrap aligned region100 v0+2096
;; @06af                               v258 = iadd v256, v257
;; @06b0                               v259 = load.i32 notrap aligned region101 v0+2112
;; @06b3                               v260 = iadd v258, v259
;; @06b4                               v261 = load.i32 notrap aligned region75 v0+2128
;; @06b7                               v262 = iadd v260, v261
;; @06b8                               v263 = load.i32 notrap aligned region102 v0+2144
;; @06bb                               v264 = iadd v262, v263
;; @06bc                               v265 = load.i32 notrap aligned region103 v0+2160
;; @06bf                               v266 = iadd v264, v265
;; @06c0                               v267 = load.i32 notrap aligned region104 v0+2176
;; @06c3                               v268 = iadd v266, v267
;; @06c4                               v269 = load.i32 notrap aligned region105 v0+2192
;; @06c7                               v270 = iadd v268, v269
;; @06c8                               v271 = load.i32 notrap aligned region106 v0+2208
;; @06cb                               v272 = iadd v270, v271
;; @06cc                               v273 = load.i32 notrap aligned region82 v0+2224
;; @06cf                               v274 = iadd v272, v273
;; @06d0                               v275 = load.i32 notrap aligned region98 v0+2240
;; @06d3                               v276 = iadd v274, v275
;; @06d4                               v277 = load.i32 notrap aligned region107 v0+2256
;; @06d7                               v278 = iadd v276, v277
;; @06d8                               v279 = load.i32 notrap aligned region108 v0+2272
;; @06db                               v280 = iadd v278, v279
;; @06dc                               v281 = load.i32 notrap aligned region109 v0+2288
;; @06df                               v282 = iadd v280, v281
;; @06e0                               v283 = load.i32 notrap aligned region110 v0+2304
;; @06e3                               v284 = iadd v282, v283
;; @06e4                               v285 = load.i32 notrap aligned region111 v0+2320
;; @06e7                               v286 = iadd v284, v285
;; @06e8                               v287 = load.i32 notrap aligned region112 v0+2336
;; @06eb                               v288 = iadd v286, v287
;; @06ec                               v289 = load.i32 notrap aligned region110 v0+2352
;; @06ef                               v290 = iadd v288, v289
;; @06f0                               v291 = load.i32 notrap aligned region98 v0+2368
;; @06f3                               v292 = iadd v290, v291
;; @06f4                               v293 = load.i32 notrap aligned region113 v0+2384
;; @06f7                               v294 = iadd v292, v293
;; @06f8                               v295 = load.i32 notrap aligned region6 v0+2400
;; @06fb                               v296 = iadd v294, v295
;; @06fc                               v297 = load.i32 notrap aligned region32 v0+2416
;; @06ff                               v298 = iadd v296, v297
;; @0700                               v299 = load.i32 notrap aligned region114 v0+2432
;; @0703                               v300 = iadd v298, v299
;; @0704                               v301 = load.i32 notrap aligned region27 v0+2448
;; @0707                               v302 = iadd v300, v301
;; @0708                               v303 = load.i32 notrap aligned region115 v0+2464
;; @070b                               v304 = iadd v302, v303
;; @070c                               v305 = load.i32 notrap aligned region80 v0+2480
;; @070f                               v306 = iadd v304, v305
;; @0710                               v307 = load.i32 notrap aligned region116 v0+2496
;; @0713                               v308 = iadd v306, v307
;; @0714                               v309 = load.i32 notrap aligned region117 v0+2512
;; @0717                               v310 = iadd v308, v309
;; @0718                               v311 = load.i32 notrap aligned region45 v0+2528
;; @071b                               v312 = iadd v310, v311
;; @071c                               v313 = load.i32 notrap aligned region118 v0+2544
;; @071f                               v314 = iadd v312, v313
;; @0720                               v315 = load.i32 notrap aligned region119 v0+2560
;; @0723                               v316 = iadd v314, v315
;; @0724                               v317 = load.i32 notrap aligned region113 v0+2576
;; @0727                               v318 = iadd v316, v317
;; @0728                               v319 = load.i32 notrap aligned region72 v0+2592
;; @072b                               v320 = iadd v318, v319
;; @072c                               v321 = load.i32 notrap aligned region120 v0+2608
;; @072f                               v322 = iadd v320, v321
;; @0730                               v323 = load.i32 notrap aligned region121 v0+2624
;; @0733                               v324 = iadd v322, v323
;; @0734                               v325 = load.i32 notrap aligned region122 v0+2640
;; @0737                               v326 = iadd v324, v325
;; @0738                               v327 = load.i32 notrap aligned region28 v0+2656
;; @073b                               v328 = iadd v326, v327
;; @073c                               v329 = load.i32 notrap aligned region123 v0+2672
;; @073f                               v330 = iadd v328, v329
;; @0740                               v331 = load.i32 notrap aligned region22 v0+2688
;; @0743                               v332 = iadd v330, v331
;; @0744                               v333 = load.i32 notrap aligned region124 v0+2704
;; @0747                               v334 = iadd v332, v333
;; @0748                               v335 = load.i32 notrap aligned region60 v0+2720
;; @074b                               v336 = iadd v334, v335
;; @074c                               v337 = load.i32 notrap aligned region125 v0+2736
;; @074f                               v338 = iadd v336, v337
;; @0750                               v339 = load.i32 notrap aligned region30 v0+2752
;; @0753                               v340 = iadd v338, v339
;; @0754                               v341 = load.i32 notrap aligned region51 v0+2768
;; @0757                               v342 = iadd v340, v341
;; @0758                               v343 = load.i32 notrap aligned region27 v0+2784
;; @075b                               v344 = iadd v342, v343
;; @075c                               v345 = load.i32 notrap aligned region113 v0+2800
;; @075f                               v346 = iadd v344, v345
;; @0760                               v347 = load.i32 notrap aligned region24 v0+2816
;; @0763                               v348 = iadd v346, v347
;; @0764                               v349 = load.i32 notrap aligned region126 v0+2832
;; @0767                               v350 = iadd v348, v349
;; @0768                               v351 = load.i32 notrap aligned region127 v0+2848
;; @076b                               v352 = iadd v350, v351
;; @076c                               v353 = load.i32 notrap aligned region16 v0+2864
;; @076f                               v354 = iadd v352, v353
;; @0770                               v355 = load.i32 notrap aligned region81 v0+2880
;; @0773                               v356 = iadd v354, v355
;; @0774                               v357 = load.i32 notrap aligned region128 v0+2896
;; @0777                               v358 = iadd v356, v357
;; @0778                               v359 = load.i32 notrap aligned region129 v0+2912
;; @077b                               v360 = iadd v358, v359
;; @077c                               v361 = load.i32 notrap aligned region18 v0+2928
;; @077f                               v362 = iadd v360, v361
;; @0780                               v363 = load.i32 notrap aligned region102 v0+2944
;; @0783                               v364 = iadd v362, v363
;; @0784                               v365 = load.i32 notrap aligned region107 v0+2960
;; @0787                               v366 = iadd v364, v365
;; @0788                               v367 = load.i32 notrap aligned region130 v0+2976
;; @078b                               v368 = iadd v366, v367
;; @078c                               v369 = load.i32 notrap aligned region18 v0+2992
;; @078f                               v370 = iadd v368, v369
;; @0790                               v371 = load.i32 notrap aligned region131 v0+3008
;; @0793                               v372 = iadd v370, v371
;; @0794                               v373 = load.i32 notrap aligned region107 v0+3024
;; @0797                               v374 = iadd v372, v373
;; @0798                               v375 = load.i32 notrap aligned region84 v0+3040
;; @079b                               v376 = iadd v374, v375
;; @079c                               v377 = load.i32 notrap aligned region103 v0+3056
;; @079f                               v378 = iadd v376, v377
;; @07a0                               v379 = load.i32 notrap aligned region9 v0+3072
;; @07a3                               v380 = iadd v378, v379
;; @07a4                               v381 = load.i32 notrap aligned region18 v0+3088
;; @07a7                               v382 = iadd v380, v381
;; @07a8                               v383 = load.i32 notrap aligned region17 v0+3104
;; @07ab                               v384 = iadd v382, v383
;; @07ac                               v385 = load.i32 notrap aligned region132 v0+3120
;; @07af                               v386 = iadd v384, v385
;; @07b0                               v387 = load.i32 notrap aligned region73 v0+3136
;; @07b3                               v388 = iadd v386, v387
;; @07b4                               v389 = load.i32 notrap aligned region133 v0+3152
;; @07b7                               v390 = iadd v388, v389
;; @07b8                               v391 = load.i32 notrap aligned region84 v0+3168
;; @07bb                               v392 = iadd v390, v391
;; @07bc                               v393 = load.i32 notrap aligned region134 v0+3184
;; @07bf                               v394 = iadd v392, v393
;; @07c0                               v395 = load.i32 notrap aligned region25 v0+3200
;; @07c3                               v396 = iadd v394, v395
;; @07c4                               v397 = load.i32 notrap aligned region25 v0+3216
;; @07c7                               v398 = iadd v396, v397
;; @07c8                               v399 = load.i32 notrap aligned region135 v0+3232
;; @07cb                               v400 = iadd v398, v399
;; @07cc                               v401 = load.i32 notrap aligned region136 v0+3248
;; @07cf                               v402 = iadd v400, v401
;; @07d0                               v403 = load.i32 notrap aligned region137 v0+3264
;; @07d3                               v404 = iadd v402, v403
;; @07d4                               v405 = load.i32 notrap aligned region134 v0+3280
;; @07d7                               v406 = iadd v404, v405
;; @07d8                               v407 = load.i32 notrap aligned region138 v0+3296
;; @07db                               v408 = iadd v406, v407
;; @07dc                               v409 = load.i32 notrap aligned region74 v0+3312
;; @07df                               v410 = iadd v408, v409
;; @07e0                               v411 = load.i32 notrap aligned region139 v0+3328
;; @07e3                               v412 = iadd v410, v411
;; @07e4                               v413 = load.i32 notrap aligned region140 v0+3344
;; @07e7                               v414 = iadd v412, v413
;; @07e8                               v415 = load.i32 notrap aligned region141 v0+3360
;; @07eb                               v416 = iadd v414, v415
;; @07ec                               v417 = load.i32 notrap aligned region52 v0+3376
;; @07ef                               v418 = iadd v416, v417
;; @07f0                               v419 = load.i32 notrap aligned region97 v0+3392
;; @07f3                               v420 = iadd v418, v419
;; @07f4                               v421 = load.i32 notrap aligned region136 v0+3408
;; @07f7                               v422 = iadd v420, v421
;; @07f8                               v423 = load.i32 notrap aligned region142 v0+3424
;; @07fb                               v424 = iadd v422, v423
;; @07fc                               v425 = load.i32 notrap aligned region82 v0+3440
;; @07ff                               v426 = iadd v424, v425
;; @0800                               v427 = load.i32 notrap aligned region143 v0+3456
;; @0803                               v428 = iadd v426, v427
;; @0804                               v429 = load.i32 notrap aligned region21 v0+3472
;; @0807                               v430 = iadd v428, v429
;; @0808                               v431 = load.i32 notrap aligned region104 v0+3488
;; @080b                               v432 = iadd v430, v431
;; @080c                               v433 = load.i32 notrap aligned region144 v0+3504
;; @080f                               v434 = iadd v432, v433
;; @0810                               v435 = load.i32 notrap aligned region145 v0+3520
;; @0813                               v436 = iadd v434, v435
;; @0814                               v437 = load.i32 notrap aligned region73 v0+3536
;; @0817                               v438 = iadd v436, v437
;; @0818                               v439 = load.i32 notrap aligned region26 v0+3552
;; @081b                               v440 = iadd v438, v439
;; @081c                               v441 = load.i32 notrap aligned region146 v0+3568
;; @081f                               v442 = iadd v440, v441
;; @0820                               v443 = load.i32 notrap aligned region96 v0+3584
;; @0823                               v444 = iadd v442, v443
;; @0824                               v445 = load.i32 notrap aligned region106 v0+3600
;; @0827                               v446 = iadd v444, v445
;; @0828                               v447 = load.i32 notrap aligned region147 v0+3616
;; @082b                               v448 = iadd v446, v447
;; @082c                               v449 = load.i32 notrap aligned region148 v0+3632
;; @082f                               v450 = iadd v448, v449
;; @0830                               v451 = load.i32 notrap aligned region132 v0+3648
;; @0833                               v452 = iadd v450, v451
;; @0834                               v453 = load.i32 notrap aligned region52 v0+3664
;; @0837                               v454 = iadd v452, v453
;; @0838                               v455 = load.i32 notrap aligned region128 v0+3680
;; @083b                               v456 = iadd v454, v455
;; @083c                               v457 = load.i32 notrap aligned region141 v0+3696
;; @083f                               v458 = iadd v456, v457
;; @0840                               v459 = load.i32 notrap aligned region131 v0+3712
;; @0843                               v460 = iadd v458, v459
;; @0844                               v461 = load.i32 notrap aligned region7 v0+3728
;; @0847                               v462 = iadd v460, v461
;; @0848                               v463 = load.i32 notrap aligned region149 v0+3744
;; @084b                               v464 = iadd v462, v463
;; @084c                               v465 = load.i32 notrap aligned region150 v0+3760
;; @084f                               v466 = iadd v464, v465
;; @0850                               v467 = load.i32 notrap aligned region151 v0+3776
;; @0853                               v468 = iadd v466, v467
;; @0854                               v469 = load.i32 notrap aligned region131 v0+3792
;; @0857                               v470 = iadd v468, v469
;; @0858                               v471 = load.i32 notrap aligned region152 v0+3808
;; @085b                               v472 = iadd v470, v471
;; @085c                               v473 = load.i32 notrap aligned region42 v0+3824
;; @085f                               v474 = iadd v472, v473
;; @0860                               v475 = load.i32 notrap aligned region24 v0+3840
;; @0863                               v476 = iadd v474, v475
;; @0864                               v477 = load.i32 notrap aligned region40 v0+3856
;; @0867                               v478 = iadd v476, v477
;; @0868                               v479 = load.i32 notrap aligned region44 v0+3872
;; @086b                               v480 = iadd v478, v479
;; @086c                               v481 = load.i32 notrap aligned region153 v0+3888
;; @086f                               v482 = iadd v480, v481
;; @0870                               v483 = load.i32 notrap aligned region154 v0+3904
;; @0873                               v484 = iadd v482, v483
;; @0874                               v485 = load.i32 notrap aligned region155 v0+3920
;; @0877                               v486 = iadd v484, v485
;; @0878                               v487 = load.i32 notrap aligned region156 v0+3936
;; @087b                               v488 = iadd v486, v487
;; @087c                               v489 = load.i32 notrap aligned region157 v0+3952
;; @087f                               v490 = iadd v488, v489
;; @0880                               v491 = load.i32 notrap aligned region20 v0+3968
;; @0883                               v492 = iadd v490, v491
;; @0884                               v493 = load.i32 notrap aligned region158 v0+3984
;; @0887                               v494 = iadd v492, v493
;; @0888                               v495 = load.i32 notrap aligned region50 v0+4000
;; @088b                               v496 = iadd v494, v495
;; @088c                               v497 = load.i32 notrap aligned region22 v0+4016
;; @088f                               v498 = iadd v496, v497
;; @0890                               v499 = load.i32 notrap aligned region159 v0+4032
;; @0893                               v500 = iadd v498, v499
;; @0894                               v501 = load.i32 notrap aligned region90 v0+4048
;; @0897                               v502 = iadd v500, v501
;; @0898                               v503 = load.i32 notrap aligned region160 v0+4064
;; @089b                               v504 = iadd v502, v503
;; @089c                               v505 = load.i32 notrap aligned region99 v0+4080
;; @089f                               v506 = iadd v504, v505
;; @08a0                               v507 = load.i32 notrap aligned region74 v0+4096
;; @08a3                               v508 = iadd v506, v507
;; @08a4                               v509 = load.i32 notrap aligned region161 v0+4112
;; @08a7                               v510 = iadd v508, v509
;; @08a8                               v511 = load.i32 notrap aligned region45 v0+4128
;; @08ab                               v512 = iadd v510, v511
;; @08ac                               v513 = load.i32 notrap aligned region95 v0+4144
;; @08af                               v514 = iadd v512, v513
;; @08b0                               jump block1
;;
;;                                 block1:
;; @08b0                               return v514
;; }
