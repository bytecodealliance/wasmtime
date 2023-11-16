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
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   1a:	 4883ec04             	sub	rsp, 4
;;   1e:	 44891c24             	mov	dword ptr [rsp], r11d
;;   22:	 448b5c2410           	mov	r11d, dword ptr [rsp + 0x10]
;;   27:	 4883ec04             	sub	rsp, 4
;;   2b:	 44891c24             	mov	dword ptr [rsp], r11d
;;   2f:	 4883c404             	add	rsp, 4
;;   33:	 e9eaffffff           	jmp	0x22
;;   38:	 4883c410             	add	rsp, 0x10
;;   3c:	 5d                   	pop	rbp
;;   3d:	 c3                   	ret	
