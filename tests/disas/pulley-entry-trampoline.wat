;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame_save 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xmov_fp x12
;;       xload64le_o32 x15, x0, 8
;;       xstore64le_o32 x15, 72, x12
;;       xmov x13, sp
;;       xstore64le_o32 x15, 64, x13
;;       xpcadd x14, 0x23    // target = 0x47
;;       xstore64le_o32 x15, 80, x14
;;       xstore64le_o32 sp, 0, x15
;;       call -0x3a    // target = 0x0
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x47
;;       xone x0
;;       pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;   47: xone x0
;;   49: xload64le_o32 x15, sp, 0
;;   50: xstore64le_o32 x15, 136, x0
;;   57: xzero x0
;;   59: pop_frame_restore 144, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;   5e: ret
