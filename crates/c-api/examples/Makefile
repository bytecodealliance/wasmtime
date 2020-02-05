###############################################################################
# Configuration

# Inherited from wasm-c-api/Makefile to just run C examples

WASM_FLAGS = -DWASM_API_DEBUG  # -DWASM_API_DEBUG_LOG
C_FLAGS = ${WASM_FLAGS} -Wall -Werror -ggdb -O -fsanitize=address
CC_FLAGS = -std=c++11 ${C_FLAGS}
LD_FLAGS = -fsanitize-memory-track-origins -fsanitize-memory-use-after-dtor

C_COMP = clang

WASMTIME_API_MODE = debug


# Base directories
WASMTIME_API_DIR = ..
WASM_DIR = wasm-c-api
EXAMPLE_DIR = ${WASM_DIR}/example
OUT_DIR = ${WASM_DIR}/out

# Example config
EXAMPLE_OUT = ${OUT_DIR}/example
EXAMPLES = \
  hello \
  callback \
  trap \
  start \
  reflect \
  global \
  table \
  memory \
  hostref \
  finalize \
  serialize \
  threads \
  # multi \

# Wasm config
WASM_INCLUDE = ${WASM_DIR}/include
WASM_SRC = ${WASM_DIR}/src
WASM_OUT = ${OUT_DIR}
WASM_C_LIBS = wasm-bin wasm-rust-api
WASM_CC_LIBS = $(error unsupported C++)


# Compiler config
ifeq (${WASMTIME_API_MODE},release)
  CARGO_BUILD_FLAGS = --release
else
  CARGO_BUILD_FLAGS =
endif

ifeq (${C_COMP},clang)
  CC_COMP = clang++
  LD_GROUP_START =
  LD_GROUP_END =
else ifeq (${C_COMP},gcc)
  CC_COMP = g++
  LD_GROUP_START = -Wl,--start-group
  LD_GROUP_END = -Wl,--end-group
else
  $(error C_COMP set to unknown compiler, must be clang or gcc)
endif

WASMTIME_BIN_DIR = ${WASMTIME_API_DIR}/../../target/${WASMTIME_API_MODE}
WASMTIME_C_LIB_DIR = ${WASMTIME_BIN_DIR}
WASMTIME_C_LIBS = wasmtime
WASMTIME_CC_LIBS = $(error unsupported c++)

WASMTIME_C_BINS = ${WASMTIME_C_LIBS:%=${WASMTIME_C_LIB_DIR}/lib%.a}

###############################################################################
# Examples
#
# To build Wasm APIs and run all examples:
#   make all
#
# To run only C examples:
#   make c
#
# To run only C++ examples:
#   make cc
#
# To run individual C example (e.g. hello):
#   make run-hello-c
#
# To run individual C++ example (e.g. hello):
#   make run-hello-cc
#

.PHONY: all cc c
all: cc c
cc: ${EXAMPLES:%=run-%-cc}
c: ${EXAMPLES:%=run-%-c}

# Running a C / C++ example
run-%-c: ${EXAMPLE_OUT}/%-c ${EXAMPLE_OUT}/%.wasm
	@echo ==== C ${@:run-%-c=%} ====; \
	cd ${EXAMPLE_OUT}; ./${@:run-%=%}
	@echo ==== Done ====

run-%-cc: ${EXAMPLE_OUT}/%-cc ${EXAMPLE_OUT}/%.wasm
	@echo ==== C++ ${@:run-%-cc=%} ====; \
	cd ${EXAMPLE_OUT}; ./${@:run-%=%}
	@echo ==== Done ====

# Compiling C / C++ example
${EXAMPLE_OUT}/%-c.o: ${EXAMPLE_DIR}/%.c ${WASM_INCLUDE}/wasm.h
	mkdir -p ${EXAMPLE_OUT}
	${C_COMP} -c ${C_FLAGS} -I. -I${WASM_INCLUDE} $< -o $@

${EXAMPLE_OUT}/%-cc.o: ${EXAMPLE_DIR}/%.cc ${WASM_INCLUDE}/wasm.hh
	mkdir -p ${EXAMPLE_OUT}
	${CC_COMP} -c ${CC_FLAGS} -I. -I${WASM_INCLUDE} $< -o $@

# Linking C / C++ example
.PRECIOUS: ${EXAMPLES:%=${EXAMPLE_OUT}/%-c}
${EXAMPLE_OUT}/%-c: ${EXAMPLE_OUT}/%-c.o ${WASMTIME_C_BINS}
	${CC_COMP} ${CC_FLAGS} ${LD_FLAGS} $< -o $@ \
		${LD_GROUP_START} \
		${WASMTIME_C_BINS} \
		${LD_GROUP_END} \
		-ldl -pthread

# Installing Wasm binaries
.PRECIOUS: ${EXAMPLES:%=${EXAMPLE_OUT}/%.wasm}
${EXAMPLE_OUT}/%.wasm: ${EXAMPLE_DIR}/%.wasm
	cp $< $@

###############################################################################
# Wasm C / C++ API
#
# To build both C / C++ APIs:
#   make wasm

.PHONY: wasm wasm-c wasm-cc
wasm: wasm-c wasm-cc
wasm-c: ${WASMTIME_C_BIN}
wasm-cc: ${WASMTIME_CC_BIN}

${WASMTIME_C_BINS}: CARGO_RUN
	cd ${WASMTIME_API_DIR}; cargo build --lib ${CARGO_BUILD_FLAGS}

.PHONY: CARGO_RUN
CARGO_RUN:


###############################################################################
# Clean-up

.PHONY: clean
clean:
	rm -rf ${OUT_DIR}

