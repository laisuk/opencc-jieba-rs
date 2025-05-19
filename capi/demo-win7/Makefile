# Compiler and flags
CC = cl
CXX = cl
CFLAGS = /I. /source-charset:utf-8 /execution-charset:utf-8
LDFLAGS = opencc_jieba_capi.dll.lib

# Executable targets
TARGETS = \
    use_opencc_jieba_c.exe \
    use_opencc_jieba_keyword_weight_c.exe \
    use_opencc_jieba_cpp.exe \
    use_opencc_jieba_cut.exe \
    use_opencc_jieba_keyword_extract.exe \
    use_opencc_jieba_keyword_weight.exe

# Default target
all: $(TARGETS)

# Rules for C programs
use_opencc_jieba_c.exe: use_opencc_jieba_c.c
	$(CC) $(CFLAGS) /Fe$@ $** $(LDFLAGS)

use_opencc_jieba_keyword_weight_c.exe: use_opencc_jieba_keyword_weight_c.c
	$(CC) $(CFLAGS) /Fe$@ $** $(LDFLAGS)

# Rules for C++ programs
use_opencc_jieba_cpp.exe: use_opencc_jieba_cpp.cpp
	$(CXX) $(CFLAGS) /Fe$@ $** $(LDFLAGS)

use_opencc_jieba_cut.exe: use_opencc_jieba_cut.cpp
	$(CXX) $(CFLAGS) /Fe$@ $** $(LDFLAGS)

use_opencc_jieba_keyword_extract.exe: use_opencc_jieba_keyword_extract.cpp
	$(CXX) $(CFLAGS) /Fe$@ $** $(LDFLAGS)

use_opencc_jieba_keyword_weight.exe: use_opencc_jieba_keyword_weight.cpp
	$(CXX) $(CFLAGS) /Fe$@ $** $(LDFLAGS)

# Clean rule
clean:
	del /Q $(TARGETS) *.obj *.exe *.ilk *.pdb

