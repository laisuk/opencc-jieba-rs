use opencc_jieba_rs::OpenCC;
use std::ffi::{c_char, CStr, CString};
use std::ptr;

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
    if instance.is_null() {
        return ptr::null_mut();
    }
    // Convert the instance pointer back into a reference
    let opencc = unsafe { &*instance };
    // Convert input from C string to Rust string
    let config_c_str = unsafe { CStr::from_ptr(config) };
    let config_str_slice = config_c_str.to_str().unwrap_or("");
    // let config_str = config_str_slice.to_owned();
    let input_c_str = unsafe { CStr::from_ptr(input) };
    let input_str_slice = input_c_str.to_str().unwrap_or("");
    // let input_str = input_str_slice.to_owned();
    let result = opencc.convert(input_str_slice, config_str_slice, punctuation);

    let c_result = CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_jieba_cut(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
) -> *mut *mut c_char {
    if instance.is_null() {
        return ptr::null_mut();
    }
    if input.is_null() {
        return ptr::null_mut();
    }
    let input_str = unsafe { CStr::from_ptr(input).to_str().unwrap() };

    let opencc = unsafe { &(*instance) };

    // let result = opencc.jieba.cut(input_str, hmm);
    let result = opencc.jieba_cut(input_str, hmm);

    // let mut result_ptrs: Vec<*mut c_char> = result
    //     .iter()
    //     .map(|s| CString::new(s.to_string()).unwrap().into_raw())
    //     .collect();
    //
    // result_ptrs.push(ptr::null_mut());
    //
    // let result_ptr = result_ptrs.as_mut_ptr();
    // std::mem::forget(result_ptrs);
    //
    // result_ptr
    vec_to_cstr_ptr(result)
}

#[no_mangle]
pub extern "C" fn opencc_jieba_join_str(
    strings: *mut *mut c_char,
    delimiter: *const c_char,
) -> *mut c_char {
    // Ensure delimiter is not null
    assert!(!delimiter.is_null());

    // Convert delimiter to a Rust string
    let delimiter_str = unsafe {
        CStr::from_ptr(delimiter)
            .to_str()
            .expect("Failed to convert delimiter to a Rust string")
    };

    // Create a new empty string to store the result
    let mut result = String::new();

    // Iterate through the strings until we find a null pointer
    let mut i = 0;
    loop {
        // Get the pointer to the current string
        let ptr = unsafe { *strings.offset(i) };
        // If the pointer is null, we've reached the end of the array
        if ptr.is_null() {
            break;
        }
        // Convert the pointer to a C string
        let c_str = unsafe { CStr::from_ptr(ptr) };
        // Convert the C string to a Rust string
        match c_str.to_str() {
            Ok(string) => {
                // Append the string to the result
                result.push_str(string);
                // If there's another string, append the delimiter
                if !unsafe { *strings.offset(i + 1) }.is_null() {
                    result.push_str(delimiter_str);
                }
            }
            Err(_) => {
                // Replace invalid UTF-8 byte sequence with a placeholder character
                result.push('�');
            }
        }
        i += 1;
    }

    // Convert the result to a CString and return a raw pointer to it
    CString::new(result).unwrap().into_raw()
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

    let input_str = unsafe { CStr::from_ptr(input).to_str().unwrap_or("") };
    let delimiter_str = unsafe { CStr::from_ptr(delimiter).to_str().unwrap_or("") };

    let opencc = unsafe { &(*instance) };
    let segments = opencc.jieba_cut(input_str, hmm);

    // Join directly without creating *mut *mut c_char
    let joined = segments.join(delimiter_str);

    CString::new(joined).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_jieba_zho_check(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
) -> i32 {
    if instance.is_null() {
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
    top_k: i32,
    method: *const c_char,
) -> *mut *mut c_char {
    if instance.is_null() {
        return ptr::null_mut();
    }
    if input.is_null() {
        return ptr::null_mut();
    }
    let input_str = unsafe { CStr::from_ptr(input).to_str().unwrap() };
    let method_str = unsafe { CStr::from_ptr(method).to_str().unwrap() };

    let opencc = unsafe { &(*instance) };

    let result = if method_str == "textrank" {
        opencc.keyword_extract_textrank(input_str, top_k as usize)
    } else {
        opencc.keyword_extract_tfidf(input_str, top_k as usize)
    };

    // let mut result_ptrs: Vec<*mut c_char> = result
    //     .iter()
    //     .map(|s| CString::new(s.to_string()).unwrap().into_raw())
    //     .collect();
    //
    // result_ptrs.push(ptr::null_mut());
    //
    // let result_ptr = result_ptrs.as_mut_ptr();
    // std::mem::forget(result_ptrs);
    //
    // result_ptr
    vec_to_cstr_ptr(result)
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
    // Convert input C string to Rust string
    let c_str = unsafe { CStr::from_ptr(input) };
    let input_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1, // Return error code if input conversion fails
    };
    // Convert method C string to Rust string
    let method_c_str = unsafe { CStr::from_ptr(method) };
    let method_str = match method_c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1, // Return error code if method conversion fails
    };
    // Call the Rust function that returns Vec<Keyword>
    let opencc = unsafe { &(*instance) };
    let keywords = if method_str == "textrank" {
        opencc.keyword_weight_textrank(input_str, top_k)
    } else if method_str == "tfidf" {
        opencc.keyword_weight_tfidf(input_str, top_k)
    } else {
        return -1; // Return error code if method is unrecognized
    };

    let keyword_len = keywords.len();
    unsafe { *out_len = keyword_len }; // Set the output length

    if keyword_len == 0 {
        return 0; // No keywords found
    }
    // Allocate memory for keyword strings and weights arrays
    let mut keyword_array = Vec::with_capacity(keyword_len);
    let mut weight_array = Vec::with_capacity(keyword_len);

    for keyword in keywords {
        let c_keyword = CString::new(keyword.keyword).unwrap(); // Convert Rust String to C string
        keyword_array.push(c_keyword.into_raw()); // Store the raw C string
        weight_array.push(keyword.weight); // Store the weight
    }
    // Return the pointers to the arrays
    unsafe {
        *out_keywords = keyword_array.as_mut_ptr();
        *out_weights = weight_array.as_mut_ptr();
    }
    // Prevent Rust from deallocating the arrays
    std::mem::forget(keyword_array);
    std::mem::forget(weight_array);

    0 // Success
}

#[no_mangle]
pub extern "C" fn opencc_jieba_free_keywords_and_weights(
    keywords: *mut *mut c_char,
    weights: *mut f64,
    len: usize,
) {
    if !keywords.is_null() {
        // Free the keyword strings
        unsafe {
            for i in 0..len {
                if !(*keywords.add(i)).is_null() {
                    let _ = CString::from_raw(*keywords.add(i)); // Reclaim ownership and free C string
                }
            }
            // Free the keyword array itself
            libc::free(keywords as *mut libc::c_void);
        }
    }

    if !weights.is_null() {
        // Free the weights array
        unsafe {
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
#[no_mangle]
pub extern "C" fn opencc_jieba_free_string_array(array: *mut *mut c_char) {
    let mut i = 0;
    loop {
        let ptr = unsafe { *array.offset(i) };
        if ptr.is_null() {
            break;
        }
        unsafe {
            let _ = CString::from_raw(ptr);
        }
        i += 1;
    }
    // Nullify the pointers (optional for safety)
    i = 0;
    loop {
        let ptr = unsafe { *array.offset(i) };
        if ptr.is_null() {
            break; // Exit loop if null is found
        }
        unsafe {
            *array.offset(i) = ptr::null_mut(); // Set each pointer to NULL
        }
        i += 1; // Move to the next pointer in the array
    }
}

// Helper function to convert Vec<&str> or Vec<String> to *mut *mut c_char
fn vec_to_cstr_ptr<T: AsRef<str>>(vec: Vec<T>) -> *mut *mut c_char {
    let mut result_ptrs: Vec<*mut c_char> = vec
        .iter()
        .map(|s| CString::new(s.as_ref()).unwrap().into_raw())
        .collect();

    result_ptrs.push(ptr::null_mut()); // Add null terminator
    let result_ptr = result_ptrs.as_mut_ptr();
    std::mem::forget(result_ptrs); // Prevent Rust from deallocating memory

    result_ptr
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
}
