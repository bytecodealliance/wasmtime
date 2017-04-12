" Vim syntax file
" Language:     Cretonne
" Maintainer:   Jakob Stoklund Olesen <stoklund@2pi.dk
" Last Change:  Sep 22, 2016

if version < 600
  syntax clear
elseif exists("b:current_syntax")
  finish
endif

" Disable spell checking even in comments.
" They tend to refer to weird stuff like assembler mnemonics anyway.
syn spell notoplevel

syn keyword ctonHeader test isa set
syn keyword ctonDecl function stack_slot jump_table
syn keyword ctonFilecheck check sameln nextln unordered not regex contained

syn match ctonType  /\<[bif]\d\+\(x\d\+\)\?\>/
syn match ctonEntity /\<\(v\|ss\|jt\|fn\|sig\)\d\+\>/
syn match ctonLabel /\<ebb\d+\>/
syn match ctonName /%\w\+\>/

syn match ctonNumber /-\?\<\d\+\>/
syn match ctonNumber /-\?\<0x\x\+\(\.\x*\)\(p[+-]\?\d\+\)\?\>/
syn match ctonHexSeq /#\x\+\>/

syn region ctonCommentLine start=";" end="$" contains=ctonFilecheck

hi def link ctonHeader        Keyword
hi def link ctonDecl          Keyword
hi def link ctonType          Type
hi def link ctonEntity        Identifier
hi def link ctonLabel         Label
hi def link ctonName          String
hi def link ctonNumber        Number
hi def link ctonHexSeq        Number
hi def link ctonCommentLine   Comment
hi def link ctonFilecheck     SpecialComment

let b:current_syntax = "cton"
