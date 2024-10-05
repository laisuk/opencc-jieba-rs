import ctypes
import os
import platform
from typing import List

# Determine the DLL file based on the operating system
if platform.system() == 'Windows':
    DLL_FILE = 'opencc_jieba_capi.dll'
elif platform.system() == 'Darwin':
    DLL_FILE = 'libopencc_jieba_capi.dylib'
elif platform.system() == 'Linux':
    DLL_FILE = 'libopencc_jieba_capi.so'
else:
    raise OSError("Unsupported operating system")

CONFIG_LIST = [
    "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "tw2t", "t2twp", "tw2t", "tw2tp",
    "t2hk", "hk2t", "t2jp", "jp2t"
]


class OpenCC:
    def __init__(self, config=None):
        self.config = config if config in CONFIG_LIST else "s2t"
        # Load the DLL
        dll_path = os.path.join(os.path.dirname(__file__), DLL_FILE)
        self.lib = ctypes.CDLL(dll_path)
        # Define function prototypes
        self.lib.opencc_new.restype = ctypes.c_void_p
        self.lib.opencc_new.argtypes = []
        self.lib.opencc_convert.restype = ctypes.c_char_p
        self.lib.opencc_convert.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_bool]
        self.lib.opencc_zho_check.restype = ctypes.c_int
        self.lib.opencc_zho_check.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        self.lib.opencc_free.argtypes = [ctypes.c_void_p]
        self.lib.opencc_jieba_cut.restype = ctypes.POINTER(ctypes.c_char_p)
        self.lib.opencc_jieba_cut.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_bool]
        self.lib.opencc_jieba_cut_and_join.restype = ctypes.c_char_p
        self.lib.opencc_jieba_cut_and_join.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_bool, ctypes.c_char_p]
        self.lib.opencc_string_free.argtypes = [ctypes.c_char_p]
        self.lib.opencc_free_string_array.argtypes = [ctypes.POINTER(ctypes.c_char_p)]
        self.lib.opencc_join_str.restype = ctypes.c_char_p
        self.lib.opencc_join_str.argtypes = [ctypes.POINTER(ctypes.c_char_p), ctypes.c_char_p]
        self.lib.opencc_jieba_keyword_extract.restype = ctypes.POINTER(ctypes.c_char_p)
        self.lib.opencc_jieba_keyword_extract.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p]

    def convert(self, text, punctuation=False):
        opencc = self.lib.opencc_new()
        if opencc is None:
            return text
        result = self.lib.opencc_convert(opencc, text.encode('utf-8'), self.config.encode('utf-8'), punctuation)
        self.lib.opencc_free(opencc)
        return result.decode('utf-8')

    def zho_check(self, text):
        opencc = self.lib.opencc_new()
        code = self.lib.opencc_zho_check(opencc, text.encode('utf-8'))
        self.lib.opencc_free(opencc)
        return code

    def jieba_cut(self, text, hmm=False):
        opencc = self.lib.opencc_new()
        result_ptr = self.lib.opencc_jieba_cut(opencc, text.encode('utf-8'), hmm)
        if result_ptr is None:
            self.lib.opencc_free(opencc)
            return [text]

        result = []
        i = 0
        while True:
            string_ptr = result_ptr[i]
            if string_ptr is None:
                break
            result.append(ctypes.string_at(string_ptr).decode('utf-8'))
            i += 1

        self.lib.opencc_free_string_array(result_ptr)
        self.lib.opencc_free(opencc)
        return result

    def jieba_cut_and_join(self, text, hmm=False, delimiter=", "):
        opencc = self.lib.opencc_new()
        result_ptr = self.lib.opencc_jieba_cut_and_join(opencc, text.encode('utf-8'), hmm, delimiter.encode('utf-8'))
        if result_ptr is None:
            self.lib.opencc_free(opencc)
            return text
        result = ctypes.string_at(result_ptr).decode('utf-8')
        # self.lib.opencc_string_free(result_ptr)
        self.lib.opencc_free(opencc)
        return result

    def jieba_join_str(self, strings: List[str], delimiter: str = " ") -> str:
        # Convert the list of strings to a list of c_char_p
        string_pointers = [ctypes.c_char_p(s.encode('utf-8')) for s in strings]
        # Append a NULL pointer to the end of the array
        string_pointers.append(None)
        # Convert the list of c_char_p to a ctypes pointer to c_char_p
        string_array = (ctypes.c_char_p * len(string_pointers))(*string_pointers)
        # Call the C function
        result = self.lib.opencc_join_str(string_array, delimiter.encode('utf-8'))

        return result.decode('utf-8')

    def jieba_keyword_extract_textrank(self, text, top_k=10):
        opencc = self.lib.opencc_new()
        result_ptr = self.lib.opencc_jieba_keyword_extract(opencc, text.encode('utf-8'), top_k, "textrank".encode('utf-8'))
        if result_ptr is None:
            self.lib.opencc_free(opencc)
            return [text]

        result = []
        i = 0
        while True:
            string_ptr = result_ptr[i]
            if string_ptr is None:
                break
            result.append(ctypes.string_at(string_ptr).decode('utf-8'))
            i += 1

        self.lib.opencc_free_string_array(result_ptr)
        self.lib.opencc_free(opencc)
        return result

    def jieba_keyword_extract_tfidf(self, text, top_k=10):
        opencc = self.lib.opencc_new()
        result_ptr = self.lib.opencc_jieba_keyword_extract(opencc, text.encode('utf-8'), top_k, "tfidf".encode('utf-8'))
        if result_ptr is None:
            self.lib.opencc_free(opencc)
            return [text]

        result = []
        i = 0
        while True:
            string_ptr = result_ptr[i]
            if string_ptr is None:
                break
            result.append(ctypes.string_at(string_ptr).decode('utf-8'))
            i += 1

        self.lib.opencc_free_string_array(result_ptr)
        self.lib.opencc_free(opencc)
        return result
