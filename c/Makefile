CC := clang
AR := llvm-ar

OPT_LEVEL ?= -O2
CFLAGS = --target=riscv64 -march=rv64imc_zba_zbb_zbc_zbs
CFLAGS += $(OPT_LEVEL) -fdata-sections -ffunction-sections
CFLAGS += -I deps/ckb-c-stdlib/libc

all: out build/libckb-script-ipc.a

build/ckb-script-ipc.o: ckb-script-ipc.c ckb-script-ipc.h
	$(CC) $(CFLAGS) -c ckb-script-ipc.c -o build/ckb-script-ipc.o

build/libckb-script-ipc.a: build/ckb-script-ipc.o
	$(AR) rcs build/libckb-script-ipc.a build/ckb-script-ipc.o

out:
	@mkdir -p build

STYLE := "{BasedOnStyle: Google, TabWidth: 4, IndentWidth: 4, UseTab: Never, SortIncludes: false, ColumnLimit: 120}"
fmt:
	clang-format -i -style=$(STYLE) \
		ckb-script-ipc.h \
		ckb-script-ipc.c
