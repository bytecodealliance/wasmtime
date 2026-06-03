#!/usr/bin/env bash

set -euxo pipefail

# Assemble AArch64 assembly to 32-bit hex opcode.
function assemble() {
    local asm="$1"

    echo "${asm}" \
        | clang -x assembler --target=aarch64 -march=armv8-a+sha2 - -c -o /dev/stdout \
        | llvm-objdump - -d --section=.text \
        | tail -n1 \
        | awk '/0:/ { print $2 }'
}

# Generate semantics for opcode in ASLT form.
function aslt() {
    local opcode="$1"

    {
        echo ':set impdef "Has SHA1 Crypto instructions" = TRUE'
        echo ":ast A64 0x${opcode}"
    } | asli
}

# Generate named testcase for a given assembly instruction.
function testcase() {
    local name="$1"
    local asm="$2"

    opcode=$(assemble "${asm}")
    aslt "${opcode}" > "${name}.aslt"
}

# Testcases.
# Arithmetic.
testcase    "add"           "add x4, x5, x6"
testcase    "add32"         "add w4, w5, w6"
testcase    "add_imm"       "add x7, x8, #291"
testcase    "add_shift"     "add x1, x2, x3, lsl 4"
testcase    "add_extend"    "add x15, x16, x17, uxtx"
testcase    "addc"          "adc x4, x5, x6"
testcase    "adds"          "adds x1, x2, x3"
testcase    "subs"          "subs x10, x11, x12, lsl 23"
testcase    "subsp"         "sub sp, sp, #32"

# 8-bit loads.
testcase    "ldrb"                      "ldrb w1, [x2]"
testcase    "ldrb_reg_reg"              "ldrb w1, [x2, x3]"
testcase    "ldrb_reg_scale"            "ldrb w1, [x2, x3, lsl #0]"
testcase    "ldrb_reg_scale_ext_uxtw"   "ldrb w1, [x2, w3, uxtw #0]"
testcase    "ldrb_reg_scale_ext_sxtw"   "ldrb w1, [x2, w3, sxtw #0]"
testcase    "ldrb_reg_scale_ext_sxtx"   "ldrb w1, [x2, x3, sxtx #0]"

# 16-bit loads.
testcase    "ldrh"                      "ldrh w1, [x2]"
testcase    "ldrh_reg_reg"              "ldrh w1, [x2, x3]"
testcase    "ldrh_reg_scale"            "ldrh w1, [x2, x3, lsl #0]"
testcase    "ldrh_reg_scale1"           "ldrh w1, [x2, x3, lsl #1]"
testcase    "ldrh_reg_scale_ext_uxtw"   "ldrh w1, [x2, w3, uxtw #0]"
testcase    "ldrh_reg_scale_ext_sxtw"   "ldrh w1, [x2, w3, sxtw #0]"
testcase    "ldrh_reg_scale_ext_sxtx"   "ldrh w1, [x2, x3, sxtx #0]"

# 32-bit loads.
testcase    "ldrsw_reg_reg"              "ldrsw x1, [x2, x3]"
testcase    "ldrsw_reg_scale"            "ldrsw x1, [x2, x3, lsl #0]"
testcase    "ldrsw_reg_scale2"           "ldrsw x1, [x2, x3, lsl #2]"
testcase    "ldrsw_reg_scale_ext_uxtw"   "ldrsw x1, [x2, w3, uxtw #0]"
testcase    "ldrsw_reg_scale_ext_sxtw"   "ldrsw x1, [x2, w3, sxtw #0]"
testcase    "ldrsw_reg_scale_ext_sxtx"   "ldrsw x1, [x2, x3, sxtx #0]"

# Misc.
testcase    "csel"          "csel x10, x12, x14, hs"
testcase    "ccmp"          "ccmp x22, x1, 13, eq"
testcase    "clz"           "clz x15, x3"
testcase    "ldp"           "ldp x1, x2, [x3], #128"
testcase    "stp"           "stp x1, x2, [x3], #128"
testcase    "ucvtf"         "ucvtf d0, w2"
testcase    "sha1h"         "sha1h s17, s6"
testcase    "sha1su1"       "sha1su1 v2.4s, v1.4s"
testcase    "sha1su0"       "sha1su0 v3.4s, v0.4s, v1.4s"
testcase    "mrs"           "mrs x0, nzcv"
testcase    "tbl"           "tbl v0.8b, {v0.16b}, v0.8b"
testcase    "uqsub"         "uqsub v3.4s, v1.4s, v2.4s"
