#!/bin/bash
set -ex

# Inspired from https://stackoverflow.com/a/246128
TOP="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $TOP

RUNTESTS=1
if [ "$1" == "--build-only" ]
then
  RUNTESTS=0
  shift
fi

# Prebuilt prefix allows us to do cross-compile outside of the target environment, saving time in qemu setup.
if [ "$1" = "--prebuilt-prefix" ]
then
  shift
  PREBUILT_PREFIX="$1"
  shift
fi

# If requested, make sure we are using latest revision of CKB VM
if [ "$1" = "--update-ckb-vm" ]
then
    cd $TOP/../deps/ckb-vm
    git pull origin develop
    cd $TOP
    shift
fi

if [ ! -d "$TOP/ckb-vm" ]
then
    ln -s ../deps/ckb-vm .
fi

if [ "$RUNTESTS" -eq "1" ]
then
if [ "$1" = "--coverage" ]
then
    INTERPRETER32="kcov $TOP/coverage $TOP/ckb-vm/target/$PREBUILT_PREFIX/release/examples/ckb_vm_runner --mode interpreter32"
    INTERPRETER64="kcov $TOP/coverage $TOP/ckb-vm/target/$PREBUILT_PREFIX/release/examples/ckb_vm_runner --mode interpreter64"
    ASM64="kcov $TOP/coverage $TOP/ckb-vm/target/$PREBUILT_PREFIX/release/examples/ckb_vm_runner --mode asm64"

    rm -rf $TOP/coverage

    if [ "x$PREBUILT_PREFIX" = "x" ]
    then
        # Build CKB VM binaries for testing
        cd ckb-vm
        cargo build --features=asm --example=ckb_vm_runner $BUILD_OPTIONS
    fi
else
    INTERPRETER32="$TOP/ckb-vm/target/$PREBUILT_PREFIX/release/examples/ckb_vm_runner --mode interpreter32"
    INTERPRETER64="$TOP/ckb-vm/target/$PREBUILT_PREFIX/release/examples/ckb_vm_runner --mode interpreter64"
    ASM64="$TOP/ckb-vm/target/$PREBUILT_PREFIX/release/examples/ckb_vm_runner --mode asm64"

    if [ "x$PREBUILT_PREFIX" = "x" ]
    then
        # Build CKB VM binaries for testing
        cd ckb-vm
        cargo build --features=asm --example=ckb_vm_runner --release $BUILD_OPTIONS
    fi
fi
fi

cd "$TOP"
if [ ! -d "$TOP/riscv-tests" ]
then
    ln -s ../deps/riscv-tests .
fi

# Build riscv-tests
cd "$TOP/riscv-tests"
autoconf
./configure
make isa-clang

if [ "$RUNTESTS" -eq "1" ]
then
    # Test CKB VM with riscv-tests
    # NOTE: let's stick with the simple way here since we know there won't be
    # whitespaces, otherwise shell might not be a good option here.
    for i in $(find . -regex ".*/rv32u[imac]-u-[a-z0-9_]*" | grep -v "fence_i"); do
        $INTERPRETER32 $i
    done
    for i in $(find . -regex ".*/rv64u[imac]-u-[a-z0-9_]*" | grep -v "fence_i"); do
        $INTERPRETER64 $i
    done
    for i in $(find . -regex ".*/rv64u[imac]-u-[a-z0-9_]*" | grep -v "fence_i" | grep -v "rv64ui-u-jalr"); do
        $ASM64 $i
    done
fi

# Test CKB VM with ckb-vm-arch-test
cd "$TOP"
if [ ! -d "$TOP/riscv-arch-test" ]
then
    ln -s ../deps/riscv-arch-test .
fi

cd "$TOP/ckb-vm-arch-test"

if [ "$RUNTESTS" -eq "1" ]
then
    COMPLIANCE_TARGET="simulate"
else
    COMPLIANCE_TARGET="compile"
fi

# TODO: more targets
mkdir -p work

find work -name "*.log" -delete && make RISCV_TARGET=ckb-vm XLEN=64 RISCV_DEVICE=I TARGET_SIM="$INTERPRETER64" $COMPLIANCE_TARGET
find work -name "*.log" -delete && make RISCV_TARGET=ckb-vm XLEN=64 RISCV_DEVICE=M TARGET_SIM="$INTERPRETER64" $COMPLIANCE_TARGET
find work -name "*.log" -delete && make RISCV_TARGET=ckb-vm XLEN=64 RISCV_DEVICE=C TARGET_SIM="$INTERPRETER64" $COMPLIANCE_TARGET
find work -name "*.log" -delete && make RISCV_TARGET=ckb-vm XLEN=64 RISCV_DEVICE=B TARGET_SIM="$INTERPRETER64" $COMPLIANCE_TARGET
find work -name "*.log" -delete && make RISCV_TARGET=ckb-vm XLEN=64 RISCV_DEVICE=I TARGET_SIM="$ASM64" $COMPLIANCE_TARGET
find work -name "*.log" -delete && make RISCV_TARGET=ckb-vm XLEN=64 RISCV_DEVICE=M TARGET_SIM="$ASM64" $COMPLIANCE_TARGET
find work -name "*.log" -delete && make RISCV_TARGET=ckb-vm XLEN=64 RISCV_DEVICE=C TARGET_SIM="$ASM64" $COMPLIANCE_TARGET
find work -name "*.log" -delete && make RISCV_TARGET=ckb-vm XLEN=64 RISCV_DEVICE=B TARGET_SIM="$ASM64" $COMPLIANCE_TARGET

cd "$TOP/programs"
make

cd "$TOP"
cargo test

echo "All tests are passed!"
