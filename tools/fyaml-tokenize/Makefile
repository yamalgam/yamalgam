# Makefile for fyaml-tokenize — builds vendored libfyaml via cmake
# and links a small C harness that reads YAML from stdin, outputs tokens as JSON.

FYAML_SRC   := ../../vendor/libfyaml-0.9.5
FYAML_BUILD := $(FYAML_SRC)/build
FYAML_LIB   := $(FYAML_BUILD)/libfyaml_static.a

CC      ?= cc
CFLAGS  := -std=c11 -Wall -Wextra -O2
LDFLAGS :=

# macOS doesn't need -lpthread (pthreads are built-in) but cmake's
# Threads::Threads adds it on Linux.
UNAME_S := $(shell uname -s)
ifneq ($(UNAME_S),Darwin)
	LDFLAGS += -lpthread
endif

INCLUDES := -DHAVE_CONFIG_H -I$(FYAML_BUILD) -I$(FYAML_SRC)/include \
             -I$(FYAML_SRC)/src/lib -I$(FYAML_SRC)/src/util -I$(FYAML_SRC)/src/thread

TARGET := fyaml-tokenize

.PHONY: all clean libfyaml

all: $(TARGET)

# Build libfyaml static library via cmake (only public API needed)
libfyaml: $(FYAML_LIB)

$(FYAML_LIB):
	cmake -S $(FYAML_SRC) -B $(FYAML_BUILD) \
		-DCMAKE_BUILD_TYPE=Release \
		-DBUILD_SHARED_LIBS=OFF \
		-DBUILD_TESTING=OFF \
		> /dev/null 2>&1
	cmake --build $(FYAML_BUILD) --target fyaml_static -j$$(sysctl -n hw.ncpu 2>/dev/null || nproc)

$(TARGET): main.c $(FYAML_LIB)
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ main.c $(FYAML_LIB) $(LDFLAGS)

clean:
	rm -f $(TARGET) *.o
	rm -rf $(FYAML_BUILD)
