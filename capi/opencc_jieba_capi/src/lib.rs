use opencc_jieba_rs::OpenCC;
use std::ffi::{c_char, CStr, CString};
use std::mem::size_of;
use std::ptr;

const OPENCC_JIEBA_ABI_NUMBER: u32 = 1;

#[repr(C)]
pub struct OpenccJiebaTag {
    pub word: *mut c_char,
    pub tag: *mut c_char,
}

enum KeywordMethod {
    TextRank,
    TfIdf,
}

impl KeywordMethod {
    fn parse(method: *const c_char) -> Option<Self> {
        match cstr_to_str(method)? {
            "textrank" => Some(Self::TextRank),
            "tfidf" => Some(Self::TfIdf),
            _ => None,
        }
    }
}

// === Public FFI: metadata ===

/// Returns the C ABI version number.
/// This value changes only when the C ABI is broken.
#[no_mangle]
pub extern "C" fn opencc_jieba_abi_number() -> u32 {
    OPENCC_JIEBA_ABI_NUMBER
}

/// Returns the OpenCC-Jieba version string (UTF-8, null-terminated).
/// Example: "0.7.3"
///
/// The returned pointer is valid for the lifetime of the program.
#[no_mangle]
pub extern "C" fn opencc_jieba_version_string() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

// === Public FFI: lifecycle ===

#[no_mangle]
pub extern "C" fn opencc_jieba_new() -> *mut OpenCC {
    Box::into_raw(Box::new(OpenCC::new()))
}

#[no_mangle]
pub extern "C" fn opencc_jieba_delete(instance: *mut OpenCC) {
    if instance.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(instance);
    }
}

#[deprecated(note = "Use `opencc_jieba_delete` instead")]
#[no_mangle]
pub extern "C" fn opencc_jieba_free(instance: *mut OpenCC) {
    opencc_jieba_delete(instance);
}

// === Public FFI: conversion ===

#[no_mangle]
pub extern "C" fn opencc_jieba_convert(
    instance: *const OpenCC,
    input: *const c_char,
    config: *const c_char,
    punctuation: bool,
) -> *mut c_char {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return ptr::null_mut(),
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return ptr::null_mut(),
    };
    let config_str = match cstr_to_str(config) {
        Some(config_str) => config_str,
        None => return ptr::null_mut(),
    };

    str_to_raw_c_char_strict(opencc.convert(input_str, config_str, punctuation))
}

#[no_mangle]
pub extern "C" fn opencc_jieba_zho_check(instance: *const OpenCC, input: *const c_char) -> i32 {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return -1,
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return -1,
    };

    opencc.zho_check(input_str)
}

// === Public FFI: segmentation and tagging ===

#[no_mangle]
pub extern "C" fn opencc_jieba_cut(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
) -> *mut *mut c_char {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return ptr::null_mut(),
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return ptr::null_mut(),
    };

    vec_to_cstr_ptr(opencc.jieba_cut(input_str, hmm))
}

#[no_mangle]
pub extern "C" fn opencc_jieba_cut_for_search(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
) -> *mut *mut c_char {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return ptr::null_mut(),
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return ptr::null_mut(),
    };

    vec_to_cstr_ptr(opencc.jieba_cut_for_search(input_str, hmm))
}

#[no_mangle]
pub extern "C" fn opencc_jieba_cut_all(
    instance: *const OpenCC,
    input: *const c_char,
) -> *mut *mut c_char {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return ptr::null_mut(),
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return ptr::null_mut(),
    };

    vec_to_cstr_ptr(opencc.jieba_cut_all(input_str))
}

#[no_mangle]
pub extern "C" fn opencc_jieba_cut_and_join(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
    delimiter: *const c_char,
) -> *mut c_char {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return ptr::null_mut(),
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return ptr::null_mut(),
    };
    let delimiter_str = match cstr_to_str(delimiter) {
        Some(delimiter_str) => delimiter_str,
        None => return ptr::null_mut(),
    };

    str_to_raw_c_char_strict(opencc.jieba_cut(input_str, hmm).join(delimiter_str))
}

#[no_mangle]
pub extern "C" fn opencc_jieba_tag(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
) -> *mut OpenccJiebaTag {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return ptr::null_mut(),
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return ptr::null_mut(),
    };

    vec_pair_to_tag_ptr(opencc.jieba_tag(input_str, hmm))
}

// === Public FFI: string utilities ===

#[no_mangle]
pub extern "C" fn opencc_jieba_join_str(
    strings: *const *const c_char,
    delimiter: *const c_char,
) -> *mut c_char {
    if strings.is_null() {
        return ptr::null_mut();
    }

    let delimiter_str = match cstr_to_str(delimiter) {
        Some(delimiter_str) => delimiter_str,
        None => return ptr::null_mut(),
    };

    match join_cstr_array(strings, delimiter_str) {
        Some(joined) => str_to_raw_c_char_strict(joined),
        None => ptr::null_mut(),
    }
}

// === Public FFI: keyword extraction ===

#[no_mangle]
pub extern "C" fn opencc_jieba_keywords(
    instance: *const OpenCC,
    input: *const c_char,
    top_k: usize,
    method: *const c_char,
) -> *mut *mut c_char {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return ptr::null_mut(),
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return ptr::null_mut(),
    };
    let method = match KeywordMethod::parse(method) {
        Some(method) => method,
        None => return ptr::null_mut(),
    };

    let keywords = match method {
        KeywordMethod::TextRank => opencc.keyword_extract_textrank(input_str, top_k),
        KeywordMethod::TfIdf => opencc.keyword_extract_tfidf(input_str, top_k),
    };

    vec_to_cstr_ptr(keywords)
}

#[no_mangle]
pub extern "C" fn opencc_jieba_keywords_and_weights(
    instance: *const OpenCC,
    input: *const c_char,
    top_k: usize,
    method: *const c_char,
    out_len: *mut usize,
    out_keywords: *mut *mut *mut c_char,
    out_weights: *mut *mut f64,
) -> i32 {
    if out_len.is_null() || out_keywords.is_null() || out_weights.is_null() {
        return -1;
    }

    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return -1,
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return -1,
    };
    let method = match KeywordMethod::parse(method) {
        Some(method) => method,
        None => return -1,
    };

    let keywords = match method {
        KeywordMethod::TextRank => opencc.keyword_weight_textrank(input_str, top_k),
        KeywordMethod::TfIdf => opencc.keyword_weight_tfidf(input_str, top_k),
    };

    let len = keywords.len();
    unsafe {
        *out_len = len;
        *out_keywords = ptr::null_mut();
        *out_weights = ptr::null_mut();
    }

    if len == 0 {
        return 0;
    }

    let keyword_array = unsafe { c_malloc_array::<*mut c_char>(len) };
    let weight_array = unsafe { c_malloc_array::<f64>(len) };
    if keyword_array.is_null() || weight_array.is_null() {
        unsafe {
            if !keyword_array.is_null() {
                libc::free(keyword_array as *mut libc::c_void);
            }
            if !weight_array.is_null() {
                libc::free(weight_array as *mut libc::c_void);
            }
        }
        return -1;
    }

    unsafe {
        fill_null_ptr_array(keyword_array, len);
    }

    for (index, keyword) in keywords.into_iter().enumerate() {
        let c_keyword = match CString::new(keyword.keyword) {
            Ok(c_keyword) => c_keyword.into_raw(),
            Err(_) => {
                unsafe {
                    for cleanup_index in 0..index {
                        let ptr = *keyword_array.add(cleanup_index);
                        if !ptr.is_null() {
                            let _ = CString::from_raw(ptr);
                        }
                    }
                    libc::free(keyword_array as *mut libc::c_void);
                    libc::free(weight_array as *mut libc::c_void);
                }
                return -1;
            }
        };

        unsafe {
            *keyword_array.add(index) = c_keyword;
            *weight_array.add(index) = keyword.weight;
        }
    }

    unsafe {
        *out_keywords = keyword_array;
        *out_weights = weight_array;
    }

    0
}

// === Public FFI: memory management ===

#[no_mangle]
pub extern "C" fn opencc_jieba_free_keywords_and_weights(
    keywords: *mut *mut c_char,
    weights: *mut f64,
    len: usize,
) {
    unsafe {
        if !keywords.is_null() {
            for i in 0..len {
                let ptr = *keywords.add(i);
                if !ptr.is_null() {
                    let _ = CString::from_raw(ptr);
                }
            }
            libc::free(keywords as *mut libc::c_void);
        }

        if !weights.is_null() {
            libc::free(weights as *mut libc::c_void);
        }
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_free_string_array(array: *mut *mut c_char) {
    if array.is_null() {
        return;
    }

    unsafe {
        let mut index = 0usize;
        loop {
            let ptr = *array.add(index);
            if ptr.is_null() {
                break;
            }

            let _ = CString::from_raw(ptr);
            index += 1;
        }

        libc::free(array as *mut libc::c_void);
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_free_tag_array(array: *mut OpenccJiebaTag) {
    if array.is_null() {
        return;
    }

    unsafe {
        let mut index = 0usize;
        loop {
            let item = array.add(index);
            if (*item).word.is_null() && (*item).tag.is_null() {
                break;
            }

            if !(*item).word.is_null() {
                let _ = CString::from_raw((*item).word);
            }
            if !(*item).tag.is_null() {
                let _ = CString::from_raw((*item).tag);
            }

            index += 1;
        }

        libc::free(array as *mut libc::c_void);
    }
}

// === Internal helpers ===

fn borrow_opencc<'a>(instance: *const OpenCC) -> Option<&'a OpenCC> {
    if instance.is_null() {
        None
    } else {
        Some(unsafe { &*instance })
    }
}

fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(ptr).to_str().ok() }
}

fn join_cstr_array(strings: *const *const c_char, delimiter: &str) -> Option<String> {
    let mut result = String::new();
    let mut index = 0usize;

    loop {
        let ptr = unsafe { *strings.add(index) };
        if ptr.is_null() {
            break;
        }

        let part = cstr_to_str(ptr)?;
        if index > 0 {
            result.push_str(delimiter);
        }
        result.push_str(part);
        index += 1;
    }

    Some(result)
}

unsafe fn c_malloc_array<T>(len: usize) -> *mut T {
    if len == 0 {
        return ptr::null_mut();
    }

    let bytes = len.checked_mul(size_of::<T>()).unwrap_or(0);
    if bytes == 0 {
        return ptr::null_mut();
    }

    libc::malloc(bytes) as *mut T
}

#[inline]
fn str_to_raw_c_char_lossy<T: AsRef<str>>(value: T) -> *mut c_char {
    match CString::new(value.as_ref()) {
        Ok(value) => value.into_raw(),
        Err(_) => CString::new("").unwrap().into_raw(),
    }
}

#[inline]
fn str_to_raw_c_char_strict<T: AsRef<str>>(value: T) -> *mut c_char {
    match CString::new(value.as_ref()) {
        Ok(value) => value.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

unsafe fn fill_null_ptr_array(array: *mut *mut c_char, len: usize) {
    for i in 0..len {
        *array.add(i) = ptr::null_mut();
    }
}

unsafe fn fill_null_tag_array(array: *mut OpenccJiebaTag, len: usize) {
    for i in 0..len {
        *array.add(i) = OpenccJiebaTag {
            word: ptr::null_mut(),
            tag: ptr::null_mut(),
        };
    }
}

/// Converts a vector of strings into a NULL-terminated `char**` allocated with `malloc`.
/// Returns NULL on allocation failure.
/// Any interior NUL in a string becomes an empty string.
fn vec_to_cstr_ptr<T: AsRef<str>>(items: Vec<T>) -> *mut *mut c_char {
    let len = items.len();
    let total = len + 1;

    let array = unsafe { c_malloc_array::<*mut c_char>(total) };
    if array.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        fill_null_ptr_array(array, total);
    }

    for (index, item) in items.into_iter().enumerate() {
        unsafe {
            *array.add(index) = str_to_raw_c_char_lossy(item);
        }
    }

    array
}

fn vec_pair_to_tag_ptr<TWord: AsRef<str>, TTag: AsRef<str>>(
    items: Vec<(TWord, TTag)>,
) -> *mut OpenccJiebaTag {
    let len = items.len();
    let total = len + 1;

    let array = unsafe { c_malloc_array::<OpenccJiebaTag>(total) };
    if array.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        fill_null_tag_array(array, total);
    }

    for (index, (word, tag)) in items.into_iter().enumerate() {
        unsafe {
            *array.add(index) = OpenccJiebaTag {
                word: str_to_raw_c_char_lossy(word),
                tag: str_to_raw_c_char_lossy(tag),
            };
        }
    }

    array
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_cstring(value: &str) -> *mut c_char {
        CString::new(value)
            .expect("CString conversion failed")
            .into_raw()
    }

    unsafe fn reclaim_raw_cstring(ptr: *mut c_char) {
        let _ = CString::from_raw(ptr);
    }

    unsafe fn cstr_array_to_vec_str<'a>(array: *mut *mut c_char) -> Vec<&'a str> {
        assert!(!array.is_null());

        let mut out = Vec::new();
        let mut index = 0isize;
        loop {
            let ptr = *array.offset(index);
            if ptr.is_null() {
                break;
            }
            out.push(CStr::from_ptr(ptr).to_str().expect("non-UTF8 token"));
            index += 1;
        }
        out
    }

    #[test]
    fn test_opencc_jieba_zho_check() {
        let opencc = OpenCC::new();
        let input = raw_cstring("你好，世界，欢迎");

        let result = opencc_jieba_zho_check(&opencc as *const OpenCC, input);

        unsafe {
            reclaim_raw_cstring(input);
        }

        assert_eq!(result, 2);
    }

    #[test]
    fn test_opencc_jieba_convert() {
        let opencc = OpenCC::new();
        let input = raw_cstring("意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。");
        let config = raw_cstring("s2twp");

        let result_ptr = opencc_jieba_convert(&opencc as *const OpenCC, input, config, true);
        let result = unsafe { CString::from_raw(result_ptr).to_string_lossy().into_owned() };

        unsafe {
            reclaim_raw_cstring(config);
            reclaim_raw_cstring(input);
        }

        assert_eq!(
            result,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_jieba_convert_2() {
        let opencc = opencc_jieba_new();
        let input =
            raw_cstring("豫章故郡，洪都新府。星分翼軫，地接衡廬。襟三江而帶五湖，控蠻荊而引甌越。");
        let config = raw_cstring("t2s");

        let result_ptr = opencc_jieba_convert(opencc, input, config, true);
        let result = unsafe { CString::from_raw(result_ptr).to_string_lossy().into_owned() };

        unsafe {
            reclaim_raw_cstring(config);
            reclaim_raw_cstring(input);
        }

        assert_eq!(
            result,
            "豫章故郡，洪都新府。星分翼轸，地接衡庐。襟三江而带五湖，控蛮荆而引瓯越。"
        );
        opencc_jieba_delete(opencc);
    }

    #[test]
    fn test_opencc_jieba_cut() {
        let opencc = OpenCC::new();
        let input = raw_cstring("你好，世界！");

        let result = opencc_jieba_cut(&opencc as *const OpenCC, input, true);
        let out = unsafe { cstr_array_to_vec_str(result) };

        assert_eq!(out, vec!["你好", "，", "世界", "！"]);

        unsafe {
            opencc_jieba_free_string_array(result);
            reclaim_raw_cstring(input);
        }
    }

    #[test]
    fn test_opencc_jieba_cut_and_join() {
        let opencc = OpenCC::new();
        let input = raw_cstring("你好，世界！");
        let delimiter = raw_cstring("/ ");

        let result = opencc_jieba_cut_and_join(&opencc as *const OpenCC, input, false, delimiter);
        let result_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };

        assert_eq!(result_str, "你好/ ，/ 世界/ ！");

        unsafe {
            opencc_jieba_free_string(result);
            reclaim_raw_cstring(input);
            reclaim_raw_cstring(delimiter);
        }
    }

    #[test]
    fn test_opencc_jieba_join_str() {
        let c1 = CString::new("Hello").unwrap();
        let c2 = CString::new("World").unwrap();
        let strings = vec![c1.as_ptr(), c2.as_ptr(), ptr::null()];
        let delimiter = CString::new(" ").unwrap();

        let result = opencc_jieba_join_str(strings.as_ptr(), delimiter.as_ptr());
        assert!(!result.is_null());

        let result_string = unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() };
        assert_eq!(result_string, "Hello World");

        opencc_jieba_free_string(result);
    }

    #[test]
    fn test_opencc_jieba_keyword_extract_textrank() {
        let opencc = OpenCC::new();
        let input = CString::new(include_str!("../../../src/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method = raw_cstring("textrank");

        let result = opencc_jieba_keywords(&opencc as *const OpenCC, input, 10, method);
        assert!(!result.is_null());

        let out = unsafe { cstr_array_to_vec_str(result) };
        assert!(!out.is_empty());

        unsafe {
            opencc_jieba_free_string_array(result);
            reclaim_raw_cstring(input);
            reclaim_raw_cstring(method);
        }
    }

    #[test]
    fn test_opencc_jieba_keyword_extract_tfidf() {
        let opencc = OpenCC::new();
        let input = CString::new(include_str!("../../../src/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method = raw_cstring("tfidf");

        let result = opencc_jieba_keywords(&opencc as *const OpenCC, input, 10, method);
        assert!(!result.is_null());

        let out = unsafe { cstr_array_to_vec_str(result) };
        assert!(!out.is_empty());

        unsafe {
            opencc_jieba_free_string_array(result);
            reclaim_raw_cstring(input);
            reclaim_raw_cstring(method);
        }
    }

    #[test]
    fn test_opencc_jieba_keyword_weight_textrank() {
        let opencc = OpenCC::new();
        let input = CString::new("这是一个测试文本，关键词提取演示。").unwrap();
        let method = CString::new("textrank").unwrap();
        let mut keyword_count = 0usize;
        let mut keywords: *mut *mut c_char = ptr::null_mut();
        let mut weights: *mut f64 = ptr::null_mut();

        let result = opencc_jieba_keywords_and_weights(
            &opencc as *const OpenCC,
            input.as_ptr(),
            5,
            method.as_ptr(),
            &mut keyword_count,
            &mut keywords,
            &mut weights,
        );

        assert_eq!(result, 0);
        assert!(keyword_count > 0);

        let keyword_vec: Vec<String> = unsafe {
            (0..keyword_count)
                .map(|i| {
                    CStr::from_ptr(*keywords.add(i))
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        };
        let weight_slice = unsafe { std::slice::from_raw_parts(weights, keyword_count) };

        assert_eq!(keyword_vec.len(), keyword_count);
        assert!(weight_slice.iter().all(|weight| *weight >= 0.0));

        opencc_jieba_free_keywords_and_weights(keywords, weights, keyword_count);
    }

    #[test]
    fn opencc_abi_number_is_non_zero_and_stable() {
        let abi = opencc_jieba_abi_number();
        assert!(abi > 0, "ABI number must be non-zero");
        assert_eq!(abi, 1, "Unexpected OpenCC C API ABI number");
    }

    #[test]
    fn opencc_version_string_is_valid_utf8_and_non_empty() {
        let ptr = opencc_jieba_version_string();
        assert!(!ptr.is_null(), "Version string pointer must not be null");

        let version = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .expect("Version string must be valid UTF-8");

        assert!(!version.is_empty(), "Version string must not be empty");
    }

    #[test]
    fn test_opencc_jieba_tag() {
        let instance = opencc_jieba_new();
        assert!(!instance.is_null());

        let input = CString::new("我喜歡Rust程序語言").unwrap();
        let array = opencc_jieba_tag(instance, input.as_ptr(), true);
        assert!(!array.is_null());

        let mut result = Vec::<(String, String)>::new();
        unsafe {
            let mut index = 0usize;
            loop {
                let item = array.add(index);
                if (*item).word.is_null() && (*item).tag.is_null() {
                    break;
                }

                let word = CStr::from_ptr((*item).word).to_str().unwrap().to_string();
                let tag = CStr::from_ptr((*item).tag).to_str().unwrap().to_string();
                result.push((word, tag));
                index += 1;
            }
        }

        assert!(!result.is_empty());
        assert!(result.iter().any(|(word, _)| word == "我"));
        assert!(result.iter().any(|(word, _)| word == "喜歡"));
        assert!(result.iter().any(|(word, _)| word == "Rust"));
        assert!(result.iter().all(|(_, tag)| !tag.is_empty()));

        opencc_jieba_free_tag_array(array);
        opencc_jieba_delete(instance);
    }
}
