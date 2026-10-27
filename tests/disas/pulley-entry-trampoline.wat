;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xmov_fp x13
;;       xmov x2, x0
;;       xload64le_o32 x3, x2, 8
;;       xstore64le_o32 x3, 72, x13
;;       xmov x14, sp
;;       xstore64le_o32 x3, 64, x14
;;       xpcadd x15, 0x23    // target = 0x4a
;;       xstore64le_o32 x3, 80, x15
;;       xstore64le_o32 sp, 0, x3
;;       call -0x3d    // target = 0x0
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x4a
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;   4a: xone x1
;;   4c: xload64le_o32 x3, sp, 0
;;   53: xstore64le_o32 x3, 136, x1
;;   5a: xzero x1
;;   5c: xstore64le_o32 x3, 160, x1
;;   63: xzero x0
;;   65: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;   6a: ret
