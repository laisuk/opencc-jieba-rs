# Compiler and flags
CC = gcc
CXX = g++
CFLAGS = -I . -L .
CXXFLAGS = -std=c++11 -I . -L .
LDFLAGS = -lopencc_jieba_capi -Wl,-rpath,@loader_path

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
	$(CXX) -o $@ $< $(CXXFLAGS) $(LDFLAGS)

use_opencc_jieba_cut: use_opencc_jieba_cut.cpp
	$(CXX) -o $@ $< $(CXXFLAGS) $(LDFLAGS)

use_opencc_jieba_keyword_extract: use_opencc_jieba_keyword_extract.cpp
	$(CXX) -o $@ $< $(CXXFLAGS) $(LDFLAGS)

use_opencc_jieba_keyword_weight: use_opencc_jieba_keyword_weight.cpp
	$(CXX) -o $@ $< $(CXXFLAGS) $(LDFLAGS)

# Phony targets
.PHONY: all clean

# Build all targets
all: $(TARGETS)

# Clean up generated files
clean:
	rm -f $(TARGETS)
