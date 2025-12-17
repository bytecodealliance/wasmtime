;;! target = "pulley64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       push_frame_save 128, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       xload64le_o32 x11, x0, 8
;;       xmov_fp x12
;;       xstore64le_o32 x11, 72, x12
;;       xmov x12, sp
;;       xstore64le_o32 x11, 64, x12
;;       xpcadd x13, 0x1c    // target = 0x40
;;       xstore64le_o32 x11, 80, x13
;;       call -0x33    // target = 0x0
;;       ├─╼ exception frame offset: SP = FP - 0x80
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x40
;;       xone x0
;;       pop_frame_restore 128, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;       ret
;;   40: xzero x0
;;   42: pop_frame_restore 128, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, sp, spilltmp0
;;   47: ret
