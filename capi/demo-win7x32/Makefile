# mingw32-make.exe all
# Compiler and flags
CC = gcc
CXX = g++
CFLAGS = -I . -L .
LDFLAGS = -lopencc_jieba_capi

# Targets and source files
TARGETS = use_opencc_jieba_c \
          use_opencc_jieba_keyword_weight_c \
          use_opencc_jieba_cpp \
          use_opencc_jieba_cut \
          use_opencc_jieba_keyword_extract \
          use_opencc_jieba_keyword_weight

# Rules for C programs
use_opencc_jieba_c: use_opencc_jieba_c.c
	$(CC) -o $@ $< $(CFLAGS) $(LDFLAGS)

use_opencc_jieba_keyword_weight_c: use_opencc_jieba_keyword_weight_c.c
	$(CC) -o $@ $< $(CFLAGS) $(LDFLAGS)

# Rules for C++ programs
use_opencc_jieba_cpp: use_opencc_jieba_cpp.cpp
	$(CXX) -o $@ $< $(CFLAGS) $(LDFLAGS)

use_opencc_jieba_cut: use_opencc_jieba_cut.cpp
	$(CXX) -o $@ $< $(CFLAGS) $(LDFLAGS)

use_opencc_jieba_keyword_extract: use_opencc_jieba_keyword_extract.cpp
	$(CXX) -o $@ $< $(CFLAGS) $(LDFLAGS)

use_opencc_jieba_keyword_weight: use_opencc_jieba_keyword_weight.cpp
	$(CXX) -o $@ $< $(CFLAGS) $(LDFLAGS)

# Phony targets
.PHONY: all clean

# Build all targets
all: $(TARGETS)

# Clean up generated files
clean:
	rm -f $(TARGETS)
