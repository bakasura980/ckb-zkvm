CC = clang-19
LD = ld.lld-19

# -flto enables Link Time Optimization for better cross-module optimization (~4% cycle reduction)
# May increase compile time and can occasionally cause issues with specific toolchain configurations

CFLAGS = --target=riscv64 -march=rv64imc_zba_zbb_zbc_zbs -flto \
		-O3 -g \
		-nodefaultlibs \
		-isystem ../../deps/musl/release/include \
		-fdata-sections -ffunction-sections

LDFLAGS = -static --gc-sections -L../../deps/musl/release/lib -L../../deps/compiler-rt-builtins-riscv/build \
		-lc -lgcc -lcompiler-rt

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

all: main.o sha512.o
	$(LD) $(LDFLAGS) -o main $^
	llvm-strip -s main

run:
	ckb-debugger --bin main

asm: sha512.o
	llvm-objdump-19 -d -S sha512.o

fmt:
	clang-format --style="{BasedOnStyle: Google, IndentWidth: 4}" -i *.c *.h

clean:
	rm -f main.o sha512.o main
