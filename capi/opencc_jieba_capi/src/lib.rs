use opencc_jieba_rs::OpenCC;

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
    let config_c_str = unsafe { std::ffi::CStr::from_ptr(config) };
    let config_str_slice = config_c_str.to_str().unwrap_or("");
    // let config_str = config_str_slice.to_owned();
    let input_c_str = unsafe { std::ffi::CStr::from_ptr(input) };
    let input_str_slice = input_c_str.to_str().unwrap_or("");
    // let input_str = input_str_slice.to_owned();
    let result = opencc.convert(input_str_slice, config_str_slice, punctuation);

    let c_result = std::ffi::CString::new(result).unwrap();
    c_result.into_raw()
}

#[no_mangle]
pub extern "C" fn opencc_string_free(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
        };
    }
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
    let c_str = unsafe { std::ffi::CStr::from_ptr(input) };
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
        let c_input = std::ffi::CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        // Call the function under test
        let result = opencc_zho_check(&opencc as *const OpenCC, c_input);
        // Free the allocated C string
        unsafe {
            let _ = std::ffi::CString::from_raw(c_input);
        };
        // Assert the result
        assert_eq!(result, 2); // Assuming the input string is in simplified Chinese, so the result should be 2
    }

    #[test]
    fn test_opencc_convert() {
        // Instance from Rust
        let opencc = OpenCC::new();
        let input = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
        let c_config = std::ffi::CString::new("s2twp")
            .expect("CString conversion failed")
            .into_raw();
        let c_input = std::ffi::CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        let punctuation = true;
        let result_ptr = opencc_convert(&opencc as *const OpenCC, c_input, c_config, punctuation);
        let result_str = unsafe {
            std::ffi::CString::from_raw(result_ptr)
                .to_string_lossy()
                .into_owned()
        };
        // Free the allocated C string
        unsafe {
            let _ = std::ffi::CString::from_raw(c_config);
            let _ = std::ffi::CString::from_raw(c_input);
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
        let c_config = std::ffi::CString::new("t2s")
            .expect("CString conversion failed")
            .into_raw();
        let c_input = std::ffi::CString::new(input)
            .expect("CString conversion failed")
            .into_raw();
        let punctuation = true;
        let result_ptr = opencc_convert(opencc, c_input, c_config, punctuation);
        let result_str = unsafe {
            std::ffi::CString::from_raw(result_ptr)
                .to_string_lossy()
                .into_owned()
        };
        // Free the allocated C string
        unsafe {
            let _ = std::ffi::CString::from_raw(c_config);
            let _ = std::ffi::CString::from_raw(c_input);
        };
        // Assert the result
        assert_eq!(
            result_str,
            "豫章故郡，洪都新府。星分翼轸，地接衡庐。襟三江而带五湖，控蛮荆而引瓯越。"
        );
    }
}
