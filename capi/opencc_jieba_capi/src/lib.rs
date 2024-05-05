use opencc_jieba_rs::OpenCC;
use std::ffi::{c_char, CStr, CString};

#[no_mangle]
pub extern "C" fn opencc_new() -> *mut OpenCC {
    Box::into_raw(Box::new(OpenCC::new()))
}

#[no_mangle]
pub extern "C" fn opencc_free(instance: *mut OpenCC) {
    if !instance.is_null() {
        // Convert the raw pointer back into a Box and let it drop
        unsafe {
            let _ = Box::from_raw(instance);
        };
    }
}

#[no_mangle]
pub extern "C" fn opencc_convert(
    instance: *const OpenCC,
    input: *const std::os::raw::c_char,
    config: *const std::os::raw::c_char,
    punctuation: bool,
) -> *mut std::os::raw::c_char {
    if instance.is_null() {
        return std::ptr::null_mut();
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
pub extern "C" fn opencc_string_free(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        };
    }
}

#[no_mangle]
pub extern "C" fn opencc_jieba_cut(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
) -> *mut *mut c_char {
    if instance.is_null() {
        return std::ptr::null_mut();
    }
    if input.is_null() {
        return std::ptr::null_mut();
    }
    let input_str = unsafe { CStr::from_ptr(input).to_str().unwrap() };

    let opencc = unsafe { &(*instance) };

    let result = opencc.jieba.cut(input_str, hmm);

    let mut result_ptrs: Vec<*mut c_char> = result
        .iter()
        .map(|s| CString::new(s.to_string()).unwrap().into_raw())
        .collect();

    result_ptrs.push(std::ptr::null_mut());

    let result_ptr = result_ptrs.as_mut_ptr();
    std::mem::forget(result_ptrs);

    result_ptr
}

#[no_mangle]
pub extern "C" fn opencc_free_string_array(array: *mut *mut c_char) {
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
}

#[no_mangle]
pub extern "C" fn join_str(strings: *mut *mut c_char, delimiter: *const c_char) -> *mut c_char {
    let delimiter_str = unsafe {
        assert!(!delimiter.is_null());
        CStr::from_ptr(delimiter).to_str().unwrap()
    };

    let mut result = String::new();

    let mut i = 0;
    loop {
        let ptr = unsafe { *strings.offset(i) };
        if ptr.is_null() {
            break;
        }
        let c_str = unsafe { CStr::from_ptr(ptr) };
        let string = c_str.to_str().unwrap();
        result.push_str(string);
        if !unsafe { *strings.offset(i + 1) }.is_null() {
            result.push_str(delimiter_str);
        }
        i += 1;
    }

    CString::new(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_jieba_cut_and_join(
    instance: *const OpenCC,
    input: *const c_char,
    hmm: bool,
    delimiter: *const c_char,
) -> *mut c_char {
    let result_ptr = opencc_jieba_cut(instance, input, hmm);
    let joined_ptr = join_str(result_ptr, delimiter);
    if !result_ptr.is_null() {
        opencc_free_string_array(result_ptr);
    }
    joined_ptr
}

#[no_mangle]
pub extern "C" fn opencc_zho_check(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencc_zho_check() {
        // Create a sample OpenCC instance
        let opencc = OpenCC::new();
        // Define a sample input string
        let input = "你好，世界，欢迎"; // Chinese characters meaning "Hello, world!"
                                        // Convert the input string to a C string
        let c_input = CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        // Call the function under test
        let result = opencc_zho_check(&opencc as *const OpenCC, c_input);
        // Free the allocated C string
        unsafe {
            let _ = CString::from_raw(c_input);
        };
        // Assert the result
        assert_eq!(result, 2); // Assuming the input string is in simplified Chinese, so the result should be 2
    }

    #[test]
    fn test_opencc_convert() {
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
        let result_ptr = opencc_convert(&opencc as *const OpenCC, c_input, c_config, punctuation);
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
    fn test_opencc_convert_2() {
        // Create instance from CAPI
        let opencc = opencc_new();
        let input = "豫章故郡，洪都新府。星分翼軫，地接衡廬。襟三江而帶五湖，控蠻荊而引甌越。";
        let c_config = CString::new("t2s")
            .expect("CString conversion failed")
            .into_raw();
        let c_input = CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        let punctuation = true;
        let result_ptr = opencc_convert(opencc, c_input, c_config, punctuation);
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

        // Convert result to Vec<String>
        let mut result_strings = Vec::new();
        let mut i = 0;
        loop {
            let ptr = unsafe { *result.offset(i) };
            if ptr.is_null() {
                break;
            }
            let c_str = unsafe { CString::from_raw(ptr) };
            let string = c_str.to_str().unwrap().to_owned();
            result_strings.push(string);
            i += 1;
        }
        println!("{:?}", result_strings);
        // Expected result
        let expected = vec!["你好", "，", "世界", "！"];

        // Check if result matches expected
        assert_eq!(result_strings, expected);

        // Free memory
        unsafe {
            // opencc_free_string_array(result);
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
            // opencc_string_free(result);
            let _ = CString::from_raw(input);
            let _ = CString::from_raw(delimiter);
        }
    }
}
