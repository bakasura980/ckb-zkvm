set -ex

rm -rf artifact
rm -rf artifact.tar.gz

mkdir artifact
mkdir artifact/arch
mkdir artifact/cryptography
mkdir artifact/spec

cp ckb-vm-arch-test/work/rv64i_m/B/*.elf artifact/arch
cp ckb-vm-arch-test/work/rv64i_m/C/*.elf artifact/arch
cp ckb-vm-arch-test/work/rv64i_m/I/*.elf artifact/arch
cp ckb-vm-arch-test/work/rv64i_m/M/*.elf artifact/arch

cp programs/build/release/*_ckbvm artifact/cryptography

for i in $(find riscv-tests/isa -regex ".*/rv32u[imac]-u-[a-z0-9_]*" | grep -v "fence_i"); do
    cp $i artifact/spec
done
for i in $(find riscv-tests/isa -regex ".*/rv64u[imac]-u-[a-z0-9_]*" | grep -v "fence_i" | grep -v "rv64ui-u-jalr"); do
    cp $i artifact/spec
done

tar -czvf artifact.tar.gz artifact
