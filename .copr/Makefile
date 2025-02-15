# Make targets are run from the root directory
WD := $(shell pwd)
COPR_DIR := $(WD)/.copr

# outdir is needed by the make_srpm build type on Fedora copr
# https://docs.pagure.org/copr.copr/user_documentation.html#make-srpm
outdir ?= $(COPR_DIR)
RPM_OPTS ?= --define "_sourcedir $(COPR_DIR)" --define "_specdir $(COPR_DIR)" --define "_builddir $(COPR_DIR)" --define "_srcrpmdir $(outdir)" --define "_rpmdir $(outdir)" --define "_buildrootdir $(COPR_DIR)/.build"

# Only the minimum dependencies to generate the srpm  using Copr's make_srpm
# method.
SRPM_DEPS ?= git git-archive-all

ifneq ($(shell id -u), 0)
SUDO_CMD := sudo
endif

# These dirs need to be added as safe.directory otherwise Copr's make_srpm complains
GIT_SAFE_DIRS := tests/spec_testsuite crates/c-api/wasm-c-api crates/wasi-common/WASI crates/wasi-nn/spec crates/wasi-crypto/spec

safe-dir-gitconfig:
	$(foreach SAFE_DIR,$(GIT_SAFE_DIRS),$(shell git config --global --add safe.directory $(WD)/$(SAFE_DIR)))

clean-safe-dir-gitconfig:
	$(foreach SAFE_DIR,$(GIT_SAFE_DIRS),$(shell git config --global --unset-all safe.directory $(SAFE_DIR)))

srpm-dep:
	$(SUDO_CMD) dnf -y install $(SRPM_DEPS)

prep: srpm-dep
	$(MAKE) -f $(COPR_DIR)/Makefile safe-dir-gitconfig
	git-archive-all --prefix=wasmtime-HEAD/ --force-submodules $(COPR_DIR)/wasmtime-HEAD.tar.gz
	$(MAKE) -f $(COPR_DIR)/Makefile clean-safe-dir-gitconfig

spec-generate: srpm-dep
	sed 's/\#GIT_COMMIT_ID\#/$(shell git rev-parse --short HEAD)/' $(COPR_DIR)/wasmtime.spec.in > $(COPR_DIR)/wasmtime.spec

srpm: prep spec-generate
	rpmbuild -bs $(RPM_OPTS) $(COPR_DIR)/wasmtime.spec

rpm: prep spec-generate
	rpmbuild -bb $(RPM_OPTS) $(COPR_DIR)/wasmtime.spec

all: srpm rpm
