# We cannot use $(shell pwd), which will return unix path format on Windows,
# making it hard to use.
cur_dir = $(dir $(abspath $(firstword $(MAKEFILE_LIST))))

TOP := $(cur_dir)
# RUSTFLAGS that are likely to be tweaked by developers. For example,
# while we enable debug logs by default here, some might want to strip them
# for minimal code size / consumed cycles.
CUSTOM_RUSTFLAGS := -C debug-assertions
# RUSTFLAGS that are less likely to be tweaked by developers. Most likely
# one would want to keep the default values here.
FULL_RUSTFLAGS := -C target-feature=+zba,+zbb,+zbc,+zbs,-a $(CUSTOM_RUSTFLAGS)
# Additional cargo args to append here. For example, one can use
# make test CARGO_ARGS="-- --nocapture" so as to inspect data emitted to
# stdout in unit tests
CARGO_ARGS :=
MODE := release
# Tweak this to change the clang version to use for building C code. By default
# we use a bash script with somes heuristics to find clang in current system.
CLANG := $(shell $(TOP)/scripts/find_clang)
CLANG_OPTIMIZER := -O2
CLANG_VERSION := $(shell $(CLANG) --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n 1)
CLANG_VERSION_MAJOR := $(shell echo $(CLANG_VERSION) | cut -d. -f1)
AR := $(subst clang,llvm-ar,$(CLANG))
OBJCOPY := $(subst clang,llvm-objcopy,$(CLANG))
CLANG_FORMAT := $(subst clang,clang-format,$(CLANG))
# When this is set to some value, the generated binaries will be copied over
BUILD_DIR :=
# Generated binaries to copy. By convention, a Rust crate's directory name will
# likely match the crate name, which is also the name of the final binary.
# However if this is not the case, you can tweak this variable. As the name hints,
# more than one binary is supported here.
BINARIES := $(notdir $(shell pwd))

ifeq (release,$(MODE))
	MODE_ARGS := --release
endif

default: build test

SPHINCSPLUS_PARAMS := sphincs-shake-128f
SPHINCSPLUS_THASH := simple
SPHINCSPLUS_DIR := sphincsplus/ref
SPHINCSPLUS_SOURCES = \
	$(SPHINCSPLUS_DIR)/address.c \
	$(SPHINCSPLUS_DIR)/merkle.c \
	$(SPHINCSPLUS_DIR)/wots.c \
	$(SPHINCSPLUS_DIR)/wotsx1.c \
	$(SPHINCSPLUS_DIR)/utils.c \
	$(SPHINCSPLUS_DIR)/utilsx1.c \
	$(SPHINCSPLUS_DIR)/fors.c \
	$(SPHINCSPLUS_DIR)/sign.c
ifneq (,$(findstring shake,$(SPHINCSPLUS_PARAMS)))
	SPHINCSPLUS_SOURCES += \
		$(SPHINCSPLUS_DIR)/fips202.c \
		$(SPHINCSPLUS_DIR)/hash_shake.c \
		$(SPHINCSPLUS_DIR)/thash_shake_$(SPHINCSPLUS_THASH).c
endif
ifneq (,$(findstring haraka,$(SPHINCSPLUS_PARAMS)))
	SPHINCSPLUS_SOURCES += \
		$(SPHINCSPLUS_DIR)/haraka.c \
		$(SPHINCSPLUS_DIR)/hash_haraka.c \
		$(SPHINCSPLUS_DIR)/thash_haraka_$(SPHINCSPLUS_THASH).c
endif
ifneq (,$(findstring sha2,$(SPHINCSPLUS_PARAMS)))
	SPHINCSPLUS_SOURCES += \
		$(SPHINCSPLUS_DIR)/sha2.c \
		$(SPHINCSPLUS_DIR)/hash_sha2.c \
		$(SPHINCSPLUS_DIR)/thash_sha2_$(SPHINCSPLUS_THASH).c
endif

CFLAGS_CKBVM_BASE = --target=riscv64-unknown-elf -march=rv64imc_zba_zbb_zbc_zbs \
	$(CLANG_OPTIMIZER) \
	-fdata-sections -ffunction-sections
CFLAGS_CKBVM_SPHINCSPLUS = $(CFLAGS_CKBVM_BASE) \
	-isystem musl/release/include \
	-I sphincsplus/ref \
	-Lmusl/release/lib -Lcompiler-rt-builtins-riscv/build \
	-lc -lgcc -lcompiler-rt \
	-DPARAMS=$(SPHINCSPLUS_PARAMS) \
	-Wl,--gc-sections
# Compared to clang-18 and clang-19, clang-20 changes the default arguments passed to ld.lld. So add this line.
ifeq ($(shell test $(CLANG_VERSION_MAJOR) -ge 20; echo $$?), 0)
    CFLAGS_CKBVM_SPHINCSPLUS += -nostartfiles -nodefaultlibs
endif

CFLAGS_NATIVE_BASE = $(CLANG_OPTIMIZER) -fdata-sections -ffunction-sections
CFLAGS_NATIVE_SPHINCSPLUS = $(CFLAGS_NATIVE_BASE) \
	-I sphincsplus/ref \
	-DPARAMS=$(SPHINCSPLUS_PARAMS) \
	-Wl,--gc-sections

build: compiler-rt-builtins-riscv musl sphincsplus
	$(CLANG) -I sphincsplus/ref \
		-DPARAMS=$(SPHINCSPLUS_PARAMS) \
		-o sphincsplus_ref_sign \
		$(SPHINCSPLUS_SOURCES) \
		$(SPHINCSPLUS_DIR)/randombytes.c \
		sphincsplus_ref_sign.c
	./sphincsplus_ref_sign > sphincsplus_ref_data.h
	$(CLANG) $(CFLAGS_CKBVM_SPHINCSPLUS) -o sphincsplus_ref_ckbvm sphincsplus_ref.c $(SPHINCSPLUS_SOURCES)
	$(CLANG) $(CFLAGS_NATIVE_SPHINCSPLUS) -o sphincsplus_ref_native sphincsplus_ref.c $(SPHINCSPLUS_SOURCES)
	@set -eu; \
	if [ "x$(BUILD_DIR)" != "x" ]; then \
		for binary in $(BINARIES); do \
			echo "Copying binary $${binary}_ckbvm to build directory"; \
			cp $(TOP)/contracts/$$binary/$${binary}_ckbvm $(TOP)/$(BUILD_DIR); \
			cp $(TOP)/$(BUILD_DIR)/$${binary}_ckbvm $(TOP)/$(BUILD_DIR)/$${binary}_ckbvm.debug; \
			$(OBJCOPY) --strip-debug --strip-all $(TOP)/$(BUILD_DIR)/$${binary}_ckbvm; \
			echo "Copying binary $${binary}_native to build directory"; \
			cp $(TOP)/contracts/$$binary/$${binary}_native $(TOP)/$(BUILD_DIR); \
		done; \
	fi

compiler-rt-builtins-riscv:
	ln -s ../../../../deps/compiler-rt-builtins-riscv .
	cd compiler-rt-builtins-riscv && make CC=$(CLANG) AR=$(AR)

musl:
	ln -s ../../../../deps/musl .
	cd musl && ./ckb/build.sh

sphincsplus:
	ln -s ../../../../deps/sphincsplus .

# test, check, clippy and fmt here are provided for completeness,
# there is nothing wrong invoking cargo directly instead of make.
test:
	echo done

check:
	cargo check $(CARGO_ARGS)

clippy:
	cargo clippy $(CARGO_ARGS)

STYLE := "{BasedOnStyle: Google, TabWidth: 4, IndentWidth: 4, UseTab: Never, SortIncludes: false, ColumnLimit: 120}"
fmt:
	$(CLANG_FORMAT) -i -style=$(STYLE) *.c

# Arbitrary cargo command is supported here. For example:
#
# make cargo CARGO_CMD=expand CARGO_ARGS="--ugly"
#
# Invokes:
# cargo expand --ugly
CARGO_CMD :=
cargo:
	cargo $(CARGO_CMD) $(CARGO_ARGS)

clean:
	cargo clean

deps-clean:
	cd compiler-rt-builtins-riscv && make clean || true
	cd musl && make clean || true
	rm -rf compiler-rt-builtins-riscv musl sphincsplus || true


prepare:
	rustup target add riscv64imac-unknown-none-elf

.PHONY: build test check clippy fmt cargo clean prepare
