CC := clang
AR := llvm-ar

OPT_LEVEL ?= -O2
ENABLE_PRINTF ?= 1

CFLAGS = --target=riscv64 -march=rv64imc_zba_zbb_zbc_zbs
CFLAGS += -fno-builtin-printf -fno-builtin-memcmp -nostdinc
CFLAGS += $(OPT_LEVEL) -fdata-sections -ffunction-sections
CFLAGS += -I deps/ckb-c-stdlib/libc -I deps/ckb-c-stdlib
CFLAGS += -I ./
ifeq ($(ENABLE_PRINTF),1)
CFLAGS += -DCKB_C_STDLIB_PRINTF
endif
CFLAGS_DECLARATION_ONLY = -DCKB_STDLIB_NO_SYSCALL_IMPL -DCKB_DECLARATION_ONLY

LDFLAGS := -Wl,-static -Wl,--gc-sections -nostdlib

all: out lib examples tests


lib: out build/libckb_script_ipc.a


build/ckb_script_ipc.o: ckb_script_ipc.c ckb_script_ipc.h
	$(CC) $(CFLAGS) $(CFLAGS_DECLARATION_ONLY) -c ckb_script_ipc.c -o build/ckb_script_ipc.o

build/libckb_script_ipc.a: build/ckb_script_ipc.o
	$(AR) rcs build/libckb_script_ipc.a build/ckb_script_ipc.o

examples: build/examples/client build/examples/server


build/examples/client: examples/client.c build/libckb_script_ipc.a
	$(CC) $(CFLAGS) $(LDFLAGS) -o build/examples/client examples/client.c build/libckb_script_ipc.a

build/examples/server: examples/server.c build/libckb_script_ipc.a
	$(CC) $(CFLAGS) $(LDFLAGS) -o build/examples/server examples/server.c build/libckb_script_ipc.a

tests: build/tests/client build/tests/server

build/tests/client: tests/client.c build/libckb_script_ipc.a
	$(CC) $(CFLAGS) $(LDFLAGS) -o build/tests/client tests/client.c build/libckb_script_ipc.a

build/tests/server: tests/server.c build/libckb_script_ipc.a
	$(CC) $(CFLAGS) $(LDFLAGS) -o build/tests/server tests/server.c build/libckb_script_ipc.a

clean:
	rm -rf build

out:
	@mkdir -p build/examples
	@mkdir -p build/tests

STYLE := "{BasedOnStyle: Google, TabWidth: 4, IndentWidth: 4, UseTab: Never, SortIncludes: false, ColumnLimit: 120}"
fmt:
	clang-format -i -style=$(STYLE) \
		ckb_script_ipc.h \
		ckb_script_ipc.c \
		examples/client.c \
		examples/server.c \
		tests/client.c \
		tests/server.c

scan-build:
	scan-build make all

