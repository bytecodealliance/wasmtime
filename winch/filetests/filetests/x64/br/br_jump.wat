;;! target = "x86_64"
(module
  (func (;0;) (result i32)
    (local i32)
    local.get 0
    loop ;; label = @1
      local.get 0
      block ;; label = @2
      end
      br 0 (;@1;)
    end
  )
  (export "" (func 0))
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   1b:	 4153                 	push	r11
;;   1d:	 448b5c2414           	mov	r11d, dword ptr [rsp + 0x14]
;;   22:	 4153                 	push	r11
;;   24:	 4883c408             	add	rsp, 8
;;   28:	 e9f0ffffff           	jmp	0x1d
;;   2d:	 4883c408             	add	rsp, 8
;;   31:	 4883c410             	add	rsp, 0x10
;;   35:	 5d                   	pop	rbp
;;   36:	 c3                   	ret	
