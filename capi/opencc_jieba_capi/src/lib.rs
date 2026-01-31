use opencc_jieba_rs::OpenCC;
use std::ffi::{c_char, CStr, CString};
use std::mem::size_of;
use std::ptr;

const OPENCC_JIEBA_ABI_NUMBER: u32 = 1;

/// Returns the C ABI version number.
/// This value changes ONLY when the C ABI is broken.
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
    // Compile-time version from Cargo.toml
    static VERSION: &str = env!("CARGO_PKG_VERSION");

    // Leak once, safe by design (process lifetime)
    static mut CSTR: *const c_char = ptr::null();

    unsafe {
        if CSTR.is_null() {
            CSTR = CString::new(VERSION).unwrap().into_raw();
        }
        CSTR
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_new() -> *mut OpenCC {
    Box::into_raw(Box::new(OpenCC::new()))
}

#[no_mangle]
pub extern "C" fn opencc_jieba_delete(instance: *mut OpenCC) {
    if !instance.is_null() {
        // Convert the raw pointer back into a Box and let it drop
        unsafe {
            let _ = Box::from_raw(instance);
        };
    }
}

#[deprecated(note = "Use `opencc_jieba_delete` instead")]
#[no_mangle]
pub extern "C" fn opencc_jieba_free(instance: *mut OpenCC) {
    if !instance.is_null() {
        // Convert the raw pointer back into a Box and let it drop
        unsafe {
            let _ = Box::from_raw(instance);
        };
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_convert(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    config: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() || input.is_null() || config.is_null() {
        return ptr::null_mut();
    }
    // Convert the instance pointer back into a reference
    let opencc = unsafe { &*instance };
    // Convert input from C string to Rust string
    let config_str_slice = match unsafe { CStr::from_ptr(config) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let input_str_slice = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let result = opencc.convert(input_str_slice, config_str_slice, punctuation);

    match CString::new(result) {
        Ok(c) => c.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_cut(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
) -> *mut *mut c_char {
    if instance.is_null() || input.is_null() {
        return ptr::null_mut();
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let opencc = unsafe { &*instance };
    let segments = opencc.jieba_cut(input_str, hmm);

    // malloc-based NULL-terminated char**
    vec_to_cstr_ptr(segments)
}

//
// Public FFI: join NULL-terminated char** into a single string
//

#[no_mangle]
pub extern "C" fn opencc_jieba_join_str(
    strings: *const *const c_char,
    delimiter: *const c_char,
) -> *mut c_char {
    if strings.is_null() || delimiter.is_null() {
        return ptr::null_mut();
    }

    let delimiter_str = match unsafe { CStr::from_ptr(delimiter) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let mut result = String::new();

    unsafe {
        let mut i = 0usize;
        loop {
            let p = *strings.add(i);
            if p.is_null() {
                break;
            }

            let part = match CStr::from_ptr(p).to_str() {
                Ok(s) => s,
                Err(_) => return ptr::null_mut(), // strict: invalid UTF-8 -> NULL
            };

            if i > 0 {
                result.push_str(delimiter_str);
            }
            result.push_str(part);
            i += 1;
        }
    }

    match CString::new(result) {
        Ok(c) => c.into_raw(),
        Err(_) => ptr::null_mut(), // should only happen if result contains '\0'
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_cut_and_join(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
    delimiter: *const c_char,
) -> *mut c_char {
    if instance.is_null() || input.is_null() || delimiter.is_null() {
        return ptr::null_mut();
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let delimiter_str = match unsafe { CStr::from_ptr(delimiter) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let opencc = unsafe { &*instance };
    let segments = opencc.jieba_cut(input_str, hmm);
    let joined = segments.join(delimiter_str);

    match CString::new(joined) {
        Ok(c) => c.into_raw(),
        Err(_) => ptr::null_mut(), // joined contains '\0'
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_zho_check(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> i32 {
    if instance.is_null() || input.is_null() {
        return -1; // Return an error code if the instance pointer is null
    }
    let opencc = unsafe { &*instance }; // Convert the instance pointer back into a reference
                                        // Convert input from C string to Rust string
    let c_str = unsafe { CStr::from_ptr(input) };
    let str_slice = c_str.to_str().unwrap_or("");
    // let input_str = str_slice.to_owned();
    opencc.zho_check(str_slice)
}

#[no_mangle]
pub extern "C" fn opencc_jieba_keywords(
    instance: *const OpenCC,
    input: *const c_char,
    top_k: usize,
    method: *const c_char,
) -> *mut *mut c_char {
    if instance.is_null() || input.is_null() || method.is_null() {
        return ptr::null_mut();
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let method_str = match unsafe { CStr::from_ptr(method) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let opencc = unsafe { &*instance };

    let keywords = match method_str {
        "textrank" => opencc.keyword_extract_textrank(input_str, top_k),
        "tfidf" => opencc.keyword_extract_tfidf(input_str, top_k),
        _ => return ptr::null_mut(),
    };

    // malloc-based NULL-terminated char**
    vec_to_cstr_ptr(keywords)
}

//
// Public FFI: keywords + weights (malloc arrays, safe frees)
//

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
    if instance.is_null()
        || input.is_null()
        || method.is_null()
        || out_len.is_null()
        || out_keywords.is_null()
        || out_weights.is_null()
    {
        return -1;
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let method_str = match unsafe { CStr::from_ptr(method) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let opencc = unsafe { &*instance };

    let keywords = match method_str {
        "textrank" => opencc.keyword_weight_textrank(input_str, top_k),
        "tfidf" => opencc.keyword_weight_tfidf(input_str, top_k),
        _ => return -1,
    };

    let n = keywords.len();
    unsafe {
        *out_len = n;
        *out_keywords = ptr::null_mut();
        *out_weights = ptr::null_mut();
    }

    if n == 0 {
        return 0;
    }

    // Allocate arrays with malloc so C/free side is consistent across platforms.
    let kw_arr = unsafe { c_malloc_array::<*mut c_char>(n) };
    let wt_arr = unsafe { c_malloc_array::<f64>(n) };

    if kw_arr.is_null() || wt_arr.is_null() {
        unsafe {
            if !kw_arr.is_null() {
                libc::free(kw_arr as *mut libc::c_void);
            }
            if !wt_arr.is_null() {
                libc::free(wt_arr as *mut libc::c_void);
            }
        }
        return -1;
    }

    // Pre-fill keyword pointers with NULL so partial cleanup is safe on failure.
    unsafe {
        for i in 0..n {
            *kw_arr.add(i) = ptr::null_mut();
        }
    }

    for (i, kw) in keywords.into_iter().enumerate() {
        let c_kw = match CString::new(kw.keyword) {
            Ok(c) => c.into_raw(),
            Err(_) => {
                // interior NUL in keyword: treat as error (clean up and fail)
                unsafe {
                    // free previously created strings
                    for j in 0..i {
                        let p = *kw_arr.add(j);
                        if !p.is_null() {
                            let _ = CString::from_raw(p);
                        }
                    }
                    libc::free(kw_arr as *mut libc::c_void);
                    libc::free(wt_arr as *mut libc::c_void);
                }
                return -1;
            }
        };

        unsafe {
            *kw_arr.add(i) = c_kw;
            *wt_arr.add(i) = kw.weight;
        }
    }

    unsafe {
        *out_keywords = kw_arr;
        *out_weights = wt_arr;
    }

    0
}

#[no_mangle]
pub extern "C" fn opencc_jieba_free_keywords_and_weights(
    keywords: *mut *mut c_char,
    weights: *mut f64,
    len: usize,
) {
    unsafe {
        if !keywords.is_null() {
            for i in 0..len {
                let p = *keywords.add(i);
                if !p.is_null() {
                    let _ = CString::from_raw(p);
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
pub extern "C" fn opencc_jieba_free_string(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        };
    }
}

//
// Public FFI: free NULL-terminated char** returned by cut/keywords
//

#[no_mangle]
pub extern "C" fn opencc_jieba_free_string_array(array: *mut *mut c_char) {
    if array.is_null() {
        return;
    }

    unsafe {
        let mut i = 0usize;
        loop {
            let p = *array.add(i);
            if p.is_null() {
                break;
            }
            // Reclaim and drop the CString
            let _ = CString::from_raw(p);
            i += 1;
        }

        // Free the pointer array itself (malloc allocator)
        libc::free(array as *mut libc::c_void);
    }
}

//
// Internal helpers: C-allocator based arrays
//

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

/// Convert Vec<T: AsRef<str>> to a NULL-terminated `char**` allocated with `malloc`.
/// Returns NULL on OOM.
/// Any interior NUL in a string becomes an empty string (never panics).
fn vec_to_cstr_ptr<T: AsRef<str>>(vec: Vec<T>) -> *mut *mut c_char {
    let n = vec.len();
    let total = n + 1; // +1 for NULL terminator

    let arr = unsafe { c_malloc_array::<*mut c_char>(total) };
    if arr.is_null() {
        return ptr::null_mut();
    }

    // Pre-fill with NULL so free() is always safe on partial failure.
    unsafe {
        for i in 0..total {
            *arr.add(i) = ptr::null_mut();
        }
    }

    for (i, s) in vec.into_iter().enumerate() {
        let c = match CString::new(s.as_ref()) {
            Ok(c) => c.into_raw(),
            Err(_) => CString::new("").unwrap().into_raw(), // interior NUL: fallback
        };
        unsafe {
            *arr.add(i) = c;
        }
    }

    // NULL terminator already set.
    arr
}

#[allow(dead_code)]
fn cstr_ptr_to_vec(keyword: *mut *mut c_char) -> Vec<String> {
    // Convert result to Vec<String>
    let mut result_strings = Vec::new();
    let mut i = 0;
    loop {
        let ptr = unsafe { *keyword.offset(i) };
        if ptr.is_null() {
            break;
        }
        let c_str = unsafe { CString::from_raw(ptr) };
        let string = c_str.to_str().unwrap().to_owned();
        result_strings.push(string);
        i += 1;
    }
    result_strings
}

#[allow(dead_code)]
fn cstr_ptr_to_vec_borrowed(keyword: *mut *mut c_char) -> Vec<String> {
    let mut result_strings = Vec::new();
    let mut i = 0;
    loop {
        let ptr = unsafe { *keyword.offset(i) };
        if ptr.is_null() {
            break;
        }
        let c_str = unsafe { CStr::from_ptr(ptr) };
        let string = c_str.to_str().unwrap().to_owned();
        result_strings.push(string);
        i += 1;
    }
    result_strings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencc_jieba_zho_check() {
        // Create a sample OpenCC instance
        let opencc = OpenCC::new();
        // Define a sample input string
        let input = "你好，世界，欢迎"; // Chinese characters meaning "Hello, world!"
                                        // Convert the input string to a C string
        let c_input = CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        // Call the function under test
        let result = opencc_jieba_zho_check(&opencc as *const OpenCC, c_input);
        // Free the allocated C string
        unsafe {
            let _ = CString::from_raw(c_input);
        };
        // Assert the result
        assert_eq!(result, 2); // Assuming the input string is in simplified Chinese, so the result should be 2
    }

    #[test]
    fn test_opencc_jieba_convert() {
        // Instance from Rust
        let opencc = OpenCC::new();
        let input = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        let c_config = CString::new("s2twp")
            .expect("CString conversion failed")
            .into_raw();
        let c_input = CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        let punctuation = true;
        let result_ptr =
            opencc_jieba_convert(&opencc as *const OpenCC, c_input, c_config, punctuation);
        let result_str = unsafe { CString::from_raw(result_ptr).to_string_lossy().into_owned() };
        // Free the allocated C string
        unsafe {
            let _ = CString::from_raw(c_config);
            let _ = CString::from_raw(c_input);
        };
        // Assert the result
        assert_eq!(
            result_str,
            "義大利羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。"
        );
    }

    #[test]
    fn test_opencc_jieba_convert_2() {
        // Create instance from CAPI
        let opencc = opencc_jieba_new();
        let input = "豫章故郡，洪都新府。星分翼軫，地接衡廬。襟三江而帶五湖，控蠻荊而引甌越。";
        let c_config = CString::new("t2s")
            .expect("CString conversion failed")
            .into_raw();
        let c_input = CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        let punctuation = true;
        let result_ptr = opencc_jieba_convert(opencc, c_input, c_config, punctuation);
        let result_str = unsafe { CString::from_raw(result_ptr).to_string_lossy().into_owned() };
        // Free the allocated C string
        unsafe {
            let _ = CString::from_raw(c_config);
            let _ = CString::from_raw(c_input);
        };
        // Assert the result
        assert_eq!(
            result_str,
            "豫章故郡，洪都新府。星分翼轸，地接衡庐。襟三江而带五湖，控蛮荆而引瓯越。"
        );
    }

    #[test]
    fn test_opencc_jieba_cut() {
        // Create OpenCC instance
        let opencc = OpenCC::new();
        // Input string
        let input = CString::new("你好，世界！").unwrap().into_raw();
        // Perform segmentation
        let result = opencc_jieba_cut(&opencc as *const OpenCC, input, true);
        let result_strings = cstr_ptr_to_vec_borrowed(result);
        println!("{:?}", result_strings);
        // Expected result
        let expected = vec!["你好", "，", "世界", "！"];
        // Check if result matches expected
        assert_eq!(result_strings, expected);
        // Free memory
        unsafe {
            opencc_jieba_free_string_array(result);
            let _ = CString::from_raw(input);
        }
    }

    #[test]
    fn test_opencc_jieba_cut_and_join() {
        // Create OpenCC instance
        let opencc = OpenCC::new();
        // Input string
        let input = CString::new("你好，世界！").unwrap().into_raw();
        // Delimiter
        let delimiter = CString::new("/ ").unwrap().into_raw();
        // Perform segmentation and join with delimiter
        let result = opencc_jieba_cut_and_join(&opencc as *const OpenCC, input, false, delimiter);
        // Convert result to String
        let result_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };
        println!("{}", result_str);
        // Expected result
        let expected = "你好/ ，/ 世界/ ！";
        // Check if result matches expected
        assert_eq!(result_str, expected);
        // Free memory
        unsafe {
            opencc_jieba_free_string(result);
            let _ = CString::from_raw(input);
            let _ = CString::from_raw(delimiter);
        }
    }

    #[test]
    fn test_opencc_jieba_join_str() {
        let c1 = CString::new("Hello").unwrap();
        let c2 = CString::new("World").unwrap();
        let strings = vec![
            c1.as_ptr(),
            c2.as_ptr(),
            ptr::null(), // C-style null terminator
        ];
        let delimiter = CString::new(" ").unwrap();
        let result = opencc_jieba_join_str(strings.as_ptr() as *mut _, delimiter.as_ptr());
        assert!(!result.is_null());
        // let result_string = unsafe { CString::from_raw(result).into_string().unwrap() };
        let result_string = unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() };
        assert_eq!(result_string, "Hello World");
        opencc_jieba_free_string(result);
        // No memory leaks — Rust still owns c1, c2, and delimiter
    }

    #[test]
    fn test_opencc_jieba_keyword_extract_textrank() {
        // Create OpenCC instance
        let opencc = OpenCC::new();
        // Input string
        let input = CString::new(include_str!("../../../src/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method_str = CString::new("textrank").unwrap().into_raw();
        // Perform segmentation
        let result = opencc_jieba_keywords(&opencc as *const OpenCC, input, 10, method_str);
        let result_strings = cstr_ptr_to_vec_borrowed(result);
        println!("TextRank: {:?}", result_strings);
        // Free memory
        unsafe {
            opencc_jieba_free_string_array(result);
            let _ = CString::from_raw(input);
        }
    }

    #[test]
    fn test_opencc_jieba_keyword_extract_tfidf() {
        // Create OpenCC instance
        let opencc = OpenCC::new();
        // Input string
        let input = CString::new(include_str!("../../../src/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method_str = CString::new("tfidf").unwrap().into_raw();
        // Perform segmentation
        let result = opencc_jieba_keywords(&opencc as *const OpenCC, input, 10, method_str);
        let result_strings = cstr_ptr_to_vec_borrowed(result);
        println!("TF-IDF :{:?}", result_strings);
        // Free memory
        unsafe {
            opencc_jieba_free_string_array(result);
            let _ = CString::from_raw(input);
        }
    }

    #[test]
    fn test_opencc_jieba_keyword_weight_textrank() {
        // Define input text for keyword extraction
        let input = "这是一个测试文本，关键词提取演示。";
        let top_k = 5;
        // Convert Rust string to C string
        let c_input = CString::new(input).unwrap();
        let c_input_ptr = c_input.as_ptr();
        let c_method = CString::new("textrank").unwrap();
        let c_method_ptr = c_method.as_ptr();
        // Initialize the OpenCC instance
        let opencc = OpenCC::new(); // Assuming OpenCC has a new() method
                                    // Output variables
        let mut keyword_count: usize = 0;
        let mut keywords: *mut *mut c_char = ptr::null_mut();
        let mut weights: *mut f64 = ptr::null_mut();
        // Call the FFI function
        let result = opencc_jieba_keywords_and_weights(
            &opencc as *const OpenCC, // Pass the OpenCC instance as a raw pointer
            c_input_ptr,              // Input text
            top_k,                    // Number of top keywords
            c_method_ptr,
            &mut keyword_count as *mut usize, // Output keyword count
            &mut keywords as *mut *mut *mut c_char, // Output keywords
            &mut weights as *mut *mut f64,    // Output weights
        );
        // Check if the function was successful
        assert_eq!(result, 0); // 0 indicates success
                               // Ensure that some keywords were extracted
        assert!(keyword_count > 0);
        // Convert C strings back to Rust strings and print keywords with their weights
        let keyword_vec: Vec<String> = unsafe {
            (0..keyword_count)
                .map(|i| {
                    let keyword_ptr = *keywords.add(i);
                    let rust_keyword = CStr::from_ptr(keyword_ptr).to_string_lossy().into_owned();
                    rust_keyword
                })
                .collect()
        };
        // View data without taking ownership
        let weight_slice: &[f64] = unsafe { std::slice::from_raw_parts(weights, keyword_count) };
        for (i, keyword) in keyword_vec.iter().enumerate() {
            println!("Keyword: {}, Weight: {}", keyword, weight_slice[i]);
        }
        // Now you can safely free weights from C
        opencc_jieba_free_keywords_and_weights(keywords, weights, keyword_count);
    }

    #[test]
    fn opencc_abi_number_is_non_zero_and_stable() {
        let abi = opencc_jieba_abi_number();

        // ABI must be non-zero
        assert!(abi > 0, "ABI number must be non-zero");

        // Optional: lock current ABI if you want strict guarantee
        assert_eq!(abi, 1, "Unexpected OpenCC C API ABI number");
    }

    #[test]
    fn opencc_version_string_is_valid_utf8_and_non_empty() {
        use std::ffi::CStr;

        let ptr = opencc_jieba_version_string();
        assert!(!ptr.is_null(), "Version string pointer must not be null");

        let ver = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .expect("Version string must be valid UTF-8");

        assert!(!ver.is_empty(), "Version string must not be empty");
    }
}
