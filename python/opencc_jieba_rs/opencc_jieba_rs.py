import ctypes
import os
import sys
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


class OpenCC:
    def __init__(self, config=None):
        config_list = [
            "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw", "tw2t", "t2twp", "tw2t", "tw2tp",
            "t2hk", "hk2t", "t2jp", "jp2t"
        ]
        self.config = config if config in config_list else "s2t"
        # Load the DLL
        dll_path = os.path.join(os.path.dirname(__file__), DLL_FILE)
        self.lib = ctypes.CDLL(dll_path)

        # Define function prototypes
        self.lib.opencc_jieba_new.restype = ctypes.c_void_p
        self.lib.opencc_jieba_new.argtypes = []
        self.lib.opencc_jieba_convert.restype = ctypes.c_void_p
        self.lib.opencc_jieba_convert.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_bool]
        self.lib.opencc_jieba_zho_check.restype = ctypes.c_int
        self.lib.opencc_jieba_zho_check.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        self.lib.opencc_jieba_delete.restype = None
        self.lib.opencc_jieba_delete.argtypes = [ctypes.c_void_p]
        self.lib.opencc_jieba_cut.restype = ctypes.POINTER(ctypes.c_char_p)
        self.lib.opencc_jieba_cut.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_bool]
        self.lib.opencc_jieba_cut_and_join.restype = ctypes.c_void_p
        self.lib.opencc_jieba_cut_and_join.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_bool, ctypes.c_char_p]
        self.lib.opencc_jieba_free_string.restype = None
        self.lib.opencc_jieba_free_string.argtypes = [ctypes.c_void_p]
        self.lib.opencc_jieba_free_string_array.restype = None
        self.lib.opencc_jieba_free_string_array.argtypes = [ctypes.POINTER(ctypes.c_char_p)]
        self.lib.opencc_jieba_join_str.restype = ctypes.c_void_p
        self.lib.opencc_jieba_join_str.argtypes = [ctypes.POINTER(ctypes.c_char_p), ctypes.c_char_p]
        self.lib.opencc_jieba_keywords.restype = ctypes.POINTER(ctypes.c_char_p)
        self.lib.opencc_jieba_keywords.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p]

        # Create and store the C instance
        self.opencc_instance = self.lib.opencc_jieba_new()
        if self.opencc_instance is None:
            print("Warning: Failed to initialize OpenCC C instance. Operations may not work as expected.", file=sys.stderr)

    def __del__(self):
        # Free the C instance when the Python object is garbage collected
        if hasattr(self, 'opencc_instance') and self.opencc_instance:
            if hasattr(self, 'lib') and hasattr(self.lib, 'opencc_jieba_delete'):
                self.lib.opencc_jieba_delete(self.opencc_instance)
            self.opencc_instance = None # Mark as freed

    def convert(self, text, punctuation=False):
        if self.opencc_instance is None:
            print("Error: OpenCC instance not available for convert.", file=sys.stderr)
            return text
        result = self.lib.opencc_jieba_convert(self.opencc_instance, text.encode('utf-8'), self.config.encode('utf-8'), punctuation)
        py_result = ctypes.string_at(result).decode('UTF-8')
        self.lib.opencc_jieba_free_string(result)
        return py_result

    def zho_check(self, text):
        if self.opencc_instance is None:
            print("Error: OpenCC instance not available for zho_check.", file=sys.stderr)
            return -1 # Indicate error, as the function is expected to return an int
        code = self.lib.opencc_jieba_zho_check(self.opencc_instance, text.encode('utf-8'))
        return code

    def jieba_cut(self, text, hmm=False):
        if self.opencc_instance is None:
            print("Error: OpenCC instance not available for jieba_cut.", file=sys.stderr)
            return [text]
        result_ptr = self.lib.opencc_jieba_cut(self.opencc_instance, text.encode('utf-8'), hmm)
        if result_ptr is None:
            return [text]

        result = []
        i = 0
        while True:
            string_ptr = result_ptr[i]
            if not string_ptr: # Check if the pointer is NULL
                break
            result.append(ctypes.string_at(string_ptr).decode('utf-8'))
            i += 1

        self.lib.opencc_jieba_free_string_array(result_ptr)
        return result

    def jieba_cut_and_join(self, text, hmm=False, delimiter=", "):
        if self.opencc_instance is None:
            print("Error: OpenCC instance not available for jieba_cut_and_join.", file=sys.stderr)
            return text
        result_ptr = self.lib.opencc_jieba_cut_and_join(self.opencc_instance, text.encode('utf-8'), hmm, delimiter.encode('utf-8'))
        if result_ptr is None:
            return text
        result = ctypes.string_at(result_ptr).decode('utf-8')
        self.lib.opencc_jieba_free_string(result_ptr)
        return result

    def jieba_join_str(self, strings: List[str], delimiter: str = " ") -> str:
        # Convert the list of strings to a list of c_char_p
        string_array = (ctypes.c_char_p * (len(strings) + 1))(
            *[ctypes.c_char_p(s.encode('utf-8')) for s in strings],
            ctypes.c_char_p(None)
        )
        # Call the C function
        result_ptr = self.lib.opencc_jieba_join_str(string_array, delimiter.encode('utf-8'))
        result = ctypes.string_at(result_ptr).decode('utf-8')
        self.lib.opencc_jieba_free_string(result_ptr)
        return result

    def jieba_keyword_extract_textrank(self, text, top_k=10):
        if self.opencc_instance is None:
            print("Error: OpenCC instance not available for jieba_keyword_extract_textrank.", file=sys.stderr)
            return [text]
        result_ptr = self.lib.opencc_jieba_keywords(self.opencc_instance, text.encode('utf-8'), top_k, "textrank".encode('utf-8'))
        if result_ptr is None:
            return [text]

        result = []
        i = 0
        while True:
            string_ptr = result_ptr[i]
            if not string_ptr: # Check if the pointer is NULL
                break
            result.append(ctypes.string_at(string_ptr).decode('utf-8'))
            i += 1

        self.lib.opencc_jieba_free_string_array(result_ptr)
        return result

    def jieba_keyword_extract_tfidf(self, text, top_k=10):
        if self.opencc_instance is None:
            print("Error: OpenCC instance not available for jieba_keyword_extract_tfidf.", file=sys.stderr)
            return [text]
        result_ptr = self.lib.opencc_jieba_keywords(self.opencc_instance, text.encode('utf-8'), top_k, "tfidf".encode('utf-8'))
        if result_ptr is None:
            return [text]

        result = []
        i = 0
        while True:
            string_ptr = result_ptr[i]
            if not string_ptr: # Check if the pointer is NULL
                break
            result.append(ctypes.string_at(string_ptr).decode('utf-8'))
            i += 1

        return result
