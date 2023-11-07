;;! target = "x86_64"

(module
  (func (export "")
    (local i32)
    local.get 0
    block
    end
    unreachable
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   1b:	 4153                 	push	r11
;;   1d:	 0f0b                 	ud2	
;;   1f:	 4883c408             	add	rsp, 8
;;   23:	 4883c410             	add	rsp, 0x10
;;   27:	 5d                   	pop	rbp
;;   28:	 c3                   	ret	
