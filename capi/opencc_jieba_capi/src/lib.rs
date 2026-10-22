//! C ABI for OpenCC-Jieba.
//!
//! Pointer-taking exports are unsafe for Rust callers: NULL checks cannot
//! validate pointer provenance, lifetimes, array bounds, or allocation ownership.
//! See each function's safety contract. Rust's `unsafe` qualifier does not change
//! C symbol names, calling conventions, parameter types, or struct layouts.

#![deny(clippy::missing_safety_doc, clippy::not_unsafe_ptr_arg_deref)]

use opencc_jieba_rs::{
    CustomDictMode, CustomDictSpec, DetofuLevel, DictSlot, KeywordMethod, OpenCC, UserDictEntry,
};
use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::mem::size_of;
use std::ptr;

const OPENCC_JIEBA_ABI_NUMBER: u32 = 1;

#[repr(C)]
pub struct OpenccJiebaTag {
    pub word: *mut c_char,
    pub tag: *mut c_char,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OpenccJiebaUserDictEntry {
    pub word: *const c_char,
    pub freq: usize,
    pub tag: *const c_char,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OpenccJiebaCustomPair {
    pub source: *const c_char,
    pub target: *const c_char,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OpenccJiebaCustomDictSpec {
    pub slot: u32,
    pub mode: u32,
    pub pairs: *const OpenccJiebaCustomPair,
    pub pair_count: usize,
}

const OPENCC_JIEBA_CUSTOM_DICT_APPEND: u32 = 1;
const OPENCC_JIEBA_CUSTOM_DICT_OVERRIDE: u32 = 2;

thread_local! {
    static C_API_LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[inline]
fn set_c_api_last_error(message: impl Into<String>) {
    C_API_LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = Some(message.into());
    });
}

#[inline]
fn get_c_api_last_error() -> Option<String> {
    C_API_LAST_ERROR.with(|last_error| last_error.borrow().clone())
}

#[inline]
fn clear_c_api_last_error() {
    C_API_LAST_ERROR.with(|last_error| *last_error.borrow_mut() = None);
}

// === Public FFI: metadata ===

/// Returns the C ABI version number.
/// This value changes only when the C ABI is broken.
#[no_mangle]
pub extern "C" fn opencc_jieba_abi_number() -> u32 {
    OPENCC_JIEBA_ABI_NUMBER
}

/// Returns the OpenCC-Jieba version string (UTF-8, null-terminated).
/// Includes the C API build revision, for example "0.8.1.1".
///
/// The returned pointer is valid for the lifetime of the program.
#[no_mangle]
pub extern "C" fn opencc_jieba_version_string() -> *const c_char {
    concat!(env!("OPENCC_JIEBA_CAPI_VERSION"), "\0").as_ptr() as *const c_char
}

// === Public FFI: lifecycle ===

#[no_mangle]
pub extern "C" fn opencc_jieba_new() -> *mut OpenCC {
    finish_constructor(
        OpenCC::try_new_with_user_dict_entries(&[])
            .map_err(|err| format!("Failed to initialize OpenCC-Jieba: {err}")),
    )
}

/// Creates an instance with copied Jieba user dictionary entries.
///
/// # Safety
/// If `entry_count > 0`, non-NULL `entries` must reference an aligned,
/// initialized, readable array of at least `entry_count` entries in one
/// allocation, with total byte size at most `isize::MAX`. Zero count ignores
/// `entries`; NULL with nonzero count is rejected. Each non-NULL `word` and
/// `tag` must satisfy the string pointer requirements below; NULL `tag` is optional.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// All arrays and strings must remain valid and unchanged during the call.
/// The constructor copies the data before returning. A non-NULL result must
/// be destroyed exactly once with [`opencc_jieba_delete`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_new_user_dict(
    entries: *const OpenccJiebaUserDictEntry,
    entry_count: usize,
) -> *mut OpenCC {
    finish_constructor(build_opencc_jieba(entries, entry_count, ptr::null(), 0))
}

/// Creates an instance with copied custom conversion dictionaries.
///
/// # Safety
/// If `spec_count > 0`, non-NULL `specs` must reference an aligned,
/// initialized, readable array of at least `spec_count` specifications in one
/// allocation. Each non-NULL `pairs` with nonzero `pair_count` must likewise
/// reference at least `pair_count` initialized pairs. Array byte sizes must
/// not exceed `isize::MAX`. Zero counts ignore array pointers; NULL arrays
/// with nonzero counts are rejected. Each non-NULL `source` and `target` must
/// satisfy the string pointer requirements below.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// All arrays and strings must remain valid and unchanged during the call.
/// The constructor copies the data before returning. A non-NULL result must
/// be destroyed exactly once with [`opencc_jieba_delete`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_new_custom(
    specs: *const OpenccJiebaCustomDictSpec,
    spec_count: usize,
) -> *mut OpenCC {
    finish_constructor(build_opencc_jieba(ptr::null(), 0, specs, spec_count))
}

/// Creates an instance with copied user entries and conversion dictionaries.
///
/// # Safety
/// If `entry_count > 0`, non-NULL `entries` must reference an aligned,
/// initialized, readable array of at least `entry_count` entries in one
/// allocation, with total byte size at most `isize::MAX`. Zero count ignores
/// `entries`; NULL with nonzero count is rejected. Each non-NULL `word` and
/// `tag` must satisfy the string pointer requirements below; NULL `tag` is optional.
/// If `spec_count > 0`, non-NULL `specs` must reference an aligned,
/// initialized, readable array of at least `spec_count` specifications in one
/// allocation. Each non-NULL `pairs` with nonzero `pair_count` must likewise
/// reference at least `pair_count` initialized pairs. Array byte sizes must
/// not exceed `isize::MAX`. Zero counts ignore array pointers; NULL arrays
/// with nonzero counts are rejected. Each non-NULL `source` and `target` must
/// satisfy the string pointer requirements below.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// All arrays and strings must remain valid and unchanged during the call.
/// The constructor copies the data before returning. A non-NULL result must
/// be destroyed exactly once with [`opencc_jieba_delete`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_new_user_dict_custom(
    entries: *const OpenccJiebaUserDictEntry,
    entry_count: usize,
    specs: *const OpenccJiebaCustomDictSpec,
    spec_count: usize,
) -> *mut OpenCC {
    finish_constructor(build_opencc_jieba(entries, entry_count, specs, spec_count))
}

/// Destroys an instance returned by an OpenCC-Jieba constructor.
///
/// # Safety
/// `instance` must be NULL or the original, still-live pointer returned by
/// an `opencc_jieba_new*` constructor, not previously freed. This call takes
/// ownership and destroys it exactly once. No references or other accesses
/// to the instance may be active during or after this call.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_delete(instance: *mut OpenCC) {
    if instance.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(instance);
    }
}

#[deprecated(note = "Use `opencc_jieba_delete` instead")]
/// Legacy alias for [`opencc_jieba_delete`].
///
/// # Safety
/// `instance` must be NULL or the original, still-live pointer returned by
/// an `opencc_jieba_new*` constructor, not previously freed. This call takes
/// ownership and destroys it exactly once. No references or other accesses
/// to the instance may be active during or after this call.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_free(instance: *mut OpenCC) {
    opencc_jieba_delete(instance);
}

// === Public FFI: dynamic Jieba dictionary ===

/// Adds or updates one Jieba word on an existing instance.
/// `has_freq` selects Some(freq), including zero; otherwise frequency is automatic.
/// NULL `tag` means no tag. Returns false and records a thread-local error on
/// invalid arguments or Rust validation failure; success clears stale errors.
///
/// # Safety
/// Non-NULL `instance` must be an aligned, live `OpenCC` with exclusive access
/// throughout this call. Callers must synchronize access to the same instance:
/// mutation must not race with conversion, segmentation, or any other access.
/// Non-NULL strings must be readable NUL-terminated byte sequences in one
/// allocation (at most `isize::MAX` bytes including the terminator), valid and
/// unchanged throughout this call. Strings remain caller-owned and are copied
/// as needed by Rust. NULL instance/word and invalid UTF-8 are rejected.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_add_word(
    instance: *mut OpenCC,
    word: *const c_char,
    has_freq: bool,
    freq: usize,
    tag: *const c_char,
) -> bool {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if instance.is_null() || word.is_null() {
            return Err("Invalid argument: instance/word is NULL".to_owned());
        }
        let word = unsafe { cstr_to_str(word) }.ok_or_else(|| "Invalid UTF-8 word".to_owned())?;
        let tag = if tag.is_null() {
            None
        } else {
            Some(unsafe { cstr_to_str(tag) }.ok_or_else(|| "Invalid UTF-8 tag".to_owned())?)
        };
        unsafe { &mut *instance }
            .add_word(word, has_freq.then_some(freq), tag)
            .map_err(|err| err.to_string())
    }));
    match result {
        Ok(Ok(_)) => {
            clear_c_api_last_error();
            true
        }
        Ok(Err(message)) => {
            set_c_api_last_error(message);
            false
        }
        Err(_) => {
            set_c_api_last_error("Panic while adding Jieba word");
            false
        }
    }
}

// === Public FFI: conversion ===

/// Converts a UTF-8 string using the named configuration.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned string is caller-owned and must be released exactly
/// once with [`opencc_jieba_free_string`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_convert(
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

/// Classifies Chinese text; returns -1 for NULL arguments or invalid UTF-8.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_zho_check(
    instance: *const OpenCC,
    input: *const c_char,
) -> i32 {
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

// === Public FFI: compatibility normalization ===

/// Normalizes CJK Compatibility Ideographs in a UTF-8 string.
///
/// This is the C API counterpart of [`OpenCC::normalize_compat`]. It is a
/// direction-independent preprocessing operation and does not modify the
/// instance, Jieba segmentation, conversion dictionaries, or punctuation
/// behavior.
///
/// The returned string must be released with [`opencc_jieba_free_string`].
///
/// Returns NULL and records a thread-local C API error when `instance` or
/// `input` is NULL or `input` is not valid UTF-8.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned string is caller-owned and must be released exactly
/// once with [`opencc_jieba_free_string`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_normalize_compat(
    instance: *const OpenCC,
    input: *const c_char,
) -> *mut c_char {
    unsafe {
        transform_utf8_ffi(instance, input, |opencc, input_str| {
            Ok(opencc.normalize_compat(input_str))
        })
    }
}

/// Applies the full built-in compatibility normalization pre-pass.
///
/// This is the C API counterpart of [`OpenCC::normalize_compat_extended`]. It
/// combines CJK Compatibility Ideograph normalization with the crate's curated
/// Unicode compatibility mappings, including selected radicals, glyph
/// variants, punctuation forms, and known text-extraction artifacts.
///
/// The returned string must be released with [`opencc_jieba_free_string`].
///
/// Returns NULL and records a thread-local C API error when `instance` or
/// `input` is NULL or `input` is not valid UTF-8.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned string is caller-owned and must be released exactly
/// once with [`opencc_jieba_free_string`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_normalize_compat_extended(
    instance: *const OpenCC,
    input: *const c_char,
) -> *mut c_char {
    unsafe {
        transform_utf8_ffi(instance, input, |opencc, input_str| {
            Ok(opencc.normalize_compat_extended(input_str))
        })
    }
}

// === Public FFI: DeTofu ===

/// Applies the built-in DeTofu display-compatibility fallback.
///
/// `level` uses the stable C ABI values `0..=7`, corresponding to ExtB
/// through ExtI respectively. The selected level is inclusive: the selected
/// extension and every supported later extension are eligible for replacement.
///
/// The returned string must be released with [`opencc_jieba_free_string`].
///
/// Returns NULL and records a thread-local C API error when `instance` or
/// `input` is NULL, `input` is not valid UTF-8, or `level` is not recognized.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned string is caller-owned and must be released exactly
/// once with [`opencc_jieba_free_string`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_detofu(
    instance: *const OpenCC,
    input: *const c_char,
    level: u32,
) -> *mut c_char {
    unsafe {
        transform_utf8_ffi(instance, input, |opencc, input_str| {
            let level = detofu_level_from_ffi(level)
                .ok_or_else(|| format!("Invalid DeTofu level: {level}"))?;
            Ok(opencc.detofu(input_str, level))
        })
    }
}

// === Public FFI: segmentation and tagging ===

/// Segments a UTF-8 string into a NULL-terminated token array.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned array is caller-owned, NULL-terminated, and must
/// be released exactly once with [`opencc_jieba_free_string_array`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_cut(
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

/// Segments text for search into a NULL-terminated token array.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned array is caller-owned, NULL-terminated, and must
/// be released exactly once with [`opencc_jieba_free_string_array`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_cut_for_search(
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

/// Segments text in full mode into a NULL-terminated token array.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned array is caller-owned, NULL-terminated, and must
/// be released exactly once with [`opencc_jieba_free_string_array`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_cut_all(
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

/// Segments text and joins the tokens with the supplied delimiter.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned string is caller-owned and must be released exactly
/// once with [`opencc_jieba_free_string`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_cut_and_join(
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

/// Returns tokens and their part-of-speech tags.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned array is caller-owned, terminated by an entry
/// with both pointers NULL, and must be released exactly once with
/// [`opencc_jieba_free_tag_array`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_tag(
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

/// Joins a NULL-terminated array of UTF-8 strings.
///
/// # Safety
/// Non-NULL `strings` must reference an aligned, initialized, readable
/// array of string pointers terminated by a NULL pointer, all within one
/// allocation of at most `isize::MAX` bytes. The array must remain valid and
/// unchanged throughout the call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned string is caller-owned and must be released exactly
/// once with [`opencc_jieba_free_string`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_join_str(
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

/// Extracts keywords into a NULL-terminated string array.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned array is caller-owned, NULL-terminated, and must
/// be released exactly once with [`opencc_jieba_free_string_array`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_keywords(
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
    let method = match parse_keyword_method(method) {
        Some(method) => method,
        None => return ptr::null_mut(),
    };

    let keywords = keyword_extract_ffi(opencc, input_str, top_k, method, None);
    vec_to_cstr_ptr(keywords)
}

/// Extracts keywords with an optional whitespace-separated POS filter.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// A non-NULL returned array is caller-owned, NULL-terminated, and must
/// be released exactly once with [`opencc_jieba_free_string_array`].
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_keywords_pos(
    instance: *const OpenCC,
    input: *const c_char,
    top_k: usize,
    method: *const c_char,
    allowed_pos: *const c_char,
) -> *mut *mut c_char {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return ptr::null_mut(),
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return ptr::null_mut(),
    };
    let method = match parse_keyword_method(method) {
        Some(method) => method,
        None => return ptr::null_mut(),
    };

    let keywords = with_allowed_pos_refs(allowed_pos, |allowed_pos_refs| {
        keyword_extract_ffi(opencc, input_str, top_k, method, allowed_pos_refs)
    });

    vec_to_cstr_ptr(keywords)
}

/// Extracts keywords and weights into counted arrays.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// Non-NULL `out_len`, `out_keywords`, and `out_weights` must be aligned and
/// exclusively writable for one value of their respective types. These
/// output regions must not overlap each other, the instance, or borrowed
/// string memory. NULL output pointers are rejected. On success, the returned
/// arrays and strings are caller-owned and must be freed together exactly
/// once with [`opencc_jieba_free_keywords_and_weights`] using the returned length.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_keywords_and_weights(
    instance: *const OpenCC,
    input: *const c_char,
    top_k: usize,
    method: *const c_char,
    out_len: *mut usize,
    out_keywords: *mut *mut *mut c_char,
    out_weights: *mut *mut f64,
) -> i32 {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return -1,
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return -1,
    };
    let method = match parse_keyword_method(method) {
        Some(method) => method,
        None => return -1,
    };

    keyword_weights_ffi_impl(
        opencc,
        input_str,
        top_k,
        method,
        None,
        out_len,
        out_keywords,
        out_weights,
    )
}

/// Extracts keywords and weights with an optional POS filter.
///
/// # Safety
/// Non-NULL `instance` must be aligned and point to a live, initialized
/// `OpenCC` that remains valid and is not mutated or freed during this call.
/// Every non-NULL string pointer must reference a readable NUL-terminated
/// byte sequence in one allocation (including the terminator, at most
/// `isize::MAX` bytes), unchanged and valid throughout the call.
/// Borrowed memory remains caller-owned; NULL arguments are handled as errors
/// or optional values as appropriate. UTF-8 is validated before use.
/// Non-NULL `out_len`, `out_keywords`, and `out_weights` must be aligned and
/// exclusively writable for one value of their respective types. These
/// output regions must not overlap each other, the instance, or borrowed
/// string memory. NULL output pointers are rejected. On success, the returned
/// arrays and strings are caller-owned and must be freed together exactly
/// once with [`opencc_jieba_free_keywords_and_weights`] using the returned length.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_keywords_and_weights_pos(
    instance: *const OpenCC,
    input: *const c_char,
    top_k: usize,
    method: *const c_char,
    allowed_pos: *const c_char,
    out_len: *mut usize,
    out_keywords: *mut *mut *mut c_char,
    out_weights: *mut *mut f64,
) -> i32 {
    let opencc = match borrow_opencc(instance) {
        Some(opencc) => opencc,
        None => return -1,
    };
    let input_str = match cstr_to_str(input) {
        Some(input_str) => input_str,
        None => return -1,
    };
    let method = match parse_keyword_method(method) {
        Some(method) => method,
        None => return -1,
    };

    with_allowed_pos_refs(allowed_pos, |allowed_pos_refs| {
        keyword_weights_ffi_impl(
            opencc,
            input_str,
            top_k,
            method,
            allowed_pos_refs,
            out_len,
            out_keywords,
            out_weights,
        )
    })
}

// === Public FFI: error state ===

#[no_mangle]
pub extern "C" fn opencc_jieba_last_error() -> *mut c_char {
    let message = get_c_api_last_error().unwrap_or_else(|| "No error".to_string());
    str_to_raw_c_char_strict(message)
}

#[no_mangle]
pub extern "C" fn opencc_jieba_clear_last_error() {
    clear_c_api_last_error();
}

// === Public FFI: memory management ===

/// Frees counted keyword and weight arrays returned by this API.
///
/// # Safety
/// Each non-NULL pointer must be the original, still-live corresponding
/// allocation returned by `opencc_jieba_keywords_and_weights*`. `len` must
/// equal the length returned with those allocations; the keyword elements
/// and strings must be unchanged and not previously freed. NULL pointers
/// are ignored. Each supplied allocation and contained string is consumed
/// exactly once. No references or other accesses may be active during or
/// after this call.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_free_keywords_and_weights(
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

/// Frees an owned string returned by this API; NULL is a no-op.
///
/// # Safety
/// `ptr` must be NULL or an original, still-live string allocation returned
/// by this C API, not previously freed. It must not be a borrowed version
/// string or a caller allocation. The allocation and its terminating NUL
/// position must be unchanged; no references or accesses may remain active
/// during or after this call. Ownership is consumed exactly once.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

/// Frees an owned NULL-terminated string array and its strings.
///
/// # Safety
/// `array` must be NULL or the original, still-live array returned by a
/// segmentation or keyword extraction function in this API. Its elements
/// and terminating NULL sentinel must be unchanged and not previously freed.
/// The array and every contained string are consumed exactly once. No
/// references or other accesses may be active during or after this call.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_free_string_array(array: *mut *mut c_char) {
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

/// Frees an owned tag array and its word and tag strings.
///
/// # Safety
/// `array` must be NULL or the original, still-live array returned by
/// [`opencc_jieba_tag`]. Its elements and terminal entry (both pointers NULL)
/// must be unchanged and not previously freed. The array and every contained
/// string are consumed exactly once. No references or other accesses may
/// be active during or after this call.
#[no_mangle]
pub unsafe extern "C" fn opencc_jieba_free_tag_array(array: *mut OpenccJiebaTag) {
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

/// Validates borrowed transformation inputs and manages the C API error state.
///
/// # Safety
/// Non-NULL `instance` must point to a live, aligned `OpenCC`, immutable for
/// this call. Non-NULL `input` must point to a readable NUL-terminated string
/// in one allocation, unchanged and valid for this call.
unsafe fn transform_utf8_ffi<F>(
    instance: *const OpenCC,
    input: *const c_char,
    transform: F,
) -> *mut c_char
where
    F: FnOnce(&OpenCC, &str) -> Result<String, String>,
{
    if instance.is_null() || input.is_null() {
        set_c_api_last_error("Invalid argument: instance/input is NULL");
        return ptr::null_mut();
    }

    let opencc = unsafe { &*instance };
    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(input_str) => input_str,
        Err(_) => {
            set_c_api_last_error("Invalid UTF-8 input");
            return ptr::null_mut();
        }
    };

    match transform(opencc, input_str) {
        Ok(output) => {
            clear_c_api_last_error();
            str_to_raw_c_char_strict(output)
        }
        Err(message) => {
            set_c_api_last_error(message);
            ptr::null_mut()
        }
    }
}

#[inline]
fn detofu_level_from_ffi(level: u32) -> Option<DetofuLevel> {
    match level {
        0 => Some(DetofuLevel::ExtB),
        1 => Some(DetofuLevel::ExtC),
        2 => Some(DetofuLevel::ExtD),
        3 => Some(DetofuLevel::ExtE),
        4 => Some(DetofuLevel::ExtF),
        5 => Some(DetofuLevel::ExtG),
        6 => Some(DetofuLevel::ExtH),
        7 => Some(DetofuLevel::ExtI),
        _ => None,
    }
}

fn finish_constructor(result: Result<OpenCC, String>) -> *mut OpenCC {
    match result {
        Ok(opencc) => {
            clear_c_api_last_error();
            Box::into_raw(Box::new(opencc))
        }
        Err(message) => {
            set_c_api_last_error(message);
            ptr::null_mut()
        }
    }
}

unsafe fn build_opencc_jieba(
    entries: *const OpenccJiebaUserDictEntry,
    entry_count: usize,
    specs: *const OpenccJiebaCustomDictSpec,
    spec_count: usize,
) -> Result<OpenCC, String> {
    let user_entries = parse_user_dict_entries(entries, entry_count)?;
    let custom_specs = parse_custom_dict_specs(specs, spec_count)?;

    let mut opencc = OpenCC::try_new_with_user_dict_entries(&user_entries)
        .map_err(|err| format!("Failed to initialize OpenCC-Jieba: {err}"))?;

    if !custom_specs.is_empty() {
        opencc
            .load_custom_dicts(&custom_specs)
            .map_err(|err| format!("Failed to apply custom conversion dictionaries: {err}"))?;
    }

    Ok(opencc)
}

unsafe fn parse_user_dict_entries(
    entries: *const OpenccJiebaUserDictEntry,
    entry_count: usize,
) -> Result<Vec<UserDictEntry>, String> {
    if entry_count == 0 {
        return Ok(Vec::new());
    }

    if entries.is_null() {
        return Err(format!(
            "Invalid argument: entries is NULL while entry_count is {entry_count}"
        ));
    }

    let ffi_entries = std::slice::from_raw_parts(entries, entry_count);
    let mut result = Vec::with_capacity(ffi_entries.len());

    for (index, entry) in ffi_entries.iter().enumerate() {
        if entry.word.is_null() {
            return Err(format!(
                "Invalid Jieba user dictionary entry {index}: word is NULL"
            ));
        }

        let word = CStr::from_ptr(entry.word)
            .to_str()
            .map_err(|_| {
                format!("Invalid Jieba user dictionary entry {index}: word is not valid UTF-8")
            })?
            .to_owned();

        if word.is_empty() {
            return Err(format!(
                "Invalid Jieba user dictionary entry {index}: word is empty"
            ));
        }

        let tag = if entry.tag.is_null() {
            None
        } else {
            Some(
                CStr::from_ptr(entry.tag)
                    .to_str()
                    .map_err(|_| {
                        format!(
                            "Invalid Jieba user dictionary entry {index}: tag is not valid UTF-8"
                        )
                    })?
                    .to_owned(),
            )
        };

        result.push(UserDictEntry {
            word,
            freq: entry.freq,
            tag,
        });
    }

    Ok(result)
}

unsafe fn parse_custom_dict_specs(
    specs: *const OpenccJiebaCustomDictSpec,
    spec_count: usize,
) -> Result<Vec<CustomDictSpec>, String> {
    if spec_count == 0 {
        return Ok(Vec::new());
    }

    if specs.is_null() {
        return Err(format!(
            "Invalid argument: specs is NULL while spec_count is {spec_count}"
        ));
    }

    let ffi_specs = std::slice::from_raw_parts(specs, spec_count);
    let mut result = Vec::with_capacity(ffi_specs.len());

    for (spec_index, spec) in ffi_specs.iter().enumerate() {
        let slot = dict_slot_from_ffi(spec.slot).ok_or_else(|| {
            format!(
                "Invalid custom dictionary slot {} in spec {}",
                spec.slot, spec_index
            )
        })?;

        let mode = match spec.mode {
            OPENCC_JIEBA_CUSTOM_DICT_APPEND => CustomDictMode::Append,
            OPENCC_JIEBA_CUSTOM_DICT_OVERRIDE => CustomDictMode::Override,
            _ => {
                return Err(format!(
                    "Invalid custom dictionary mode {} in spec {}",
                    spec.mode, spec_index
                ))
            }
        };

        if spec.pair_count > 0 && spec.pairs.is_null() {
            return Err(format!(
                "Invalid custom dictionary spec {}: pairs is NULL while pair_count is {}",
                spec_index, spec.pair_count
            ));
        }

        let mut pairs = Vec::with_capacity(spec.pair_count);

        if spec.pair_count > 0 {
            let ffi_pairs = std::slice::from_raw_parts(spec.pairs, spec.pair_count);

            for (pair_index, pair) in ffi_pairs.iter().enumerate() {
                if pair.source.is_null() {
                    return Err(format!(
                        "Invalid custom dictionary spec {} pair {}: source is NULL",
                        spec_index, pair_index
                    ));
                }

                if pair.target.is_null() {
                    return Err(format!(
                        "Invalid custom dictionary spec {} pair {}: target is NULL",
                        spec_index, pair_index
                    ));
                }

                let source = CStr::from_ptr(pair.source)
                    .to_str()
                    .map_err(|_| {
                        format!(
                            "Invalid custom dictionary spec {} pair {}: source is not valid UTF-8",
                            spec_index, pair_index
                        )
                    })?
                    .to_owned();

                let target = CStr::from_ptr(pair.target)
                    .to_str()
                    .map_err(|_| {
                        format!(
                            "Invalid custom dictionary spec {} pair {}: target is not valid UTF-8",
                            spec_index, pair_index
                        )
                    })?
                    .to_owned();

                pairs.push((source, target));
            }
        }

        result.push(CustomDictSpec { slot, pairs, mode });
    }

    Ok(result)
}

#[inline]
fn dict_slot_from_ffi(slot: u32) -> Option<DictSlot> {
    match slot {
        1 => Some(DictSlot::STCharacters),
        2 => Some(DictSlot::STPhrases),
        3 => Some(DictSlot::TSCharacters),
        4 => Some(DictSlot::TSPhrases),
        5 => Some(DictSlot::TWPhrases),
        6 => Some(DictSlot::TWPhrasesRev),
        7 => Some(DictSlot::HKPhrases),
        8 => Some(DictSlot::HKPhrasesRev),
        9 => Some(DictSlot::TWVariants),
        10 => Some(DictSlot::TWVariantsPhrases),
        11 => Some(DictSlot::TWVariantsRev),
        12 => Some(DictSlot::TWVariantsRevPhrases),
        13 => Some(DictSlot::HKVariants),
        14 => Some(DictSlot::HKVariantsPhrases),
        15 => Some(DictSlot::HKVariantsRev),
        16 => Some(DictSlot::HKVariantsRevPhrases),
        17 => Some(DictSlot::JPSCharacters),
        18 => Some(DictSlot::JPSCharactersRev),
        19 => Some(DictSlot::JPSPhrases),
        _ => None,
    }
}

/// # Safety
/// Non-NULL `instance` must point to a live, aligned `OpenCC`, immutable
/// for the returned borrow lifetime.
unsafe fn borrow_opencc<'a>(instance: *const OpenCC) -> Option<&'a OpenCC> {
    if instance.is_null() {
        None
    } else {
        Some(unsafe { &*instance })
    }
}

/// # Safety
/// Non-NULL `ptr` must point to a readable NUL-terminated string in one
/// allocation, unchanged and valid for the returned borrow lifetime.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(ptr).to_str().ok() }
}

/// # Safety
/// `strings` must point to a readable, aligned NULL-terminated pointer
/// array whose non-NULL strings satisfy `cstr_to_str` for this call.
unsafe fn join_cstr_array(strings: *const *const c_char, delimiter: &str) -> Option<String> {
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

// ------ POS ------ //
#[inline]
/// # Safety
/// Non-NULL `method` must satisfy `cstr_to_str` for this call.
unsafe fn parse_keyword_method(method: *const c_char) -> Option<KeywordMethod> {
    KeywordMethod::parse_str(cstr_to_str(method)?)
}

#[inline]
/// # Safety
/// Non-NULL `pos` must satisfy `cstr_to_str` for this call.
unsafe fn parse_allowed_pos(pos: *const c_char) -> Option<Vec<String>> {
    if pos.is_null() {
        return None;
    }

    let pos_str = cstr_to_str(pos)?.trim();
    if pos_str.is_empty() {
        return None;
    }

    Some(pos_str.split_whitespace().map(|s| s.to_string()).collect())
}

#[inline]
fn keyword_extract_ffi(
    opencc: &OpenCC,
    input_str: &str,
    top_k: usize,
    method: KeywordMethod,
    allowed_pos: Option<&[&str]>,
) -> Vec<String> {
    match method {
        KeywordMethod::TextRank => {
            opencc.keyword_extract_textrank_pos(input_str, top_k, allowed_pos)
        }
        KeywordMethod::TfIdf => opencc.keyword_extract_tfidf_pos(input_str, top_k, allowed_pos),
    }
}

#[inline]
fn keyword_weight_ffi(
    opencc: &OpenCC,
    input_str: &str,
    top_k: usize,
    method: KeywordMethod,
    allowed_pos: Option<&[&str]>,
) -> Vec<opencc_jieba_rs::Keyword> {
    match method {
        KeywordMethod::TextRank => {
            opencc.keyword_weight_textrank_pos(input_str, top_k, allowed_pos)
        }
        KeywordMethod::TfIdf => opencc.keyword_weight_tfidf_pos(input_str, top_k, allowed_pos),
    }
}

#[allow(clippy::too_many_arguments)]
/// # Safety
/// Non-NULL output pointers must be aligned and exclusively writable for
/// one value, with disjoint regions that do not overlap borrowed inputs.
unsafe fn keyword_weights_ffi_impl(
    opencc: &OpenCC,
    input_str: &str,
    top_k: usize,
    method: KeywordMethod,
    allowed_pos: Option<&[&str]>,
    out_len: *mut usize,
    out_keywords: *mut *mut *mut c_char,
    out_weights: *mut *mut f64,
) -> i32 {
    if out_len.is_null() || out_keywords.is_null() || out_weights.is_null() {
        return -1;
    }

    let keywords = keyword_weight_ffi(opencc, input_str, top_k, method, allowed_pos);

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

#[inline]
/// # Safety
/// Non-NULL `allowed_pos` must satisfy `cstr_to_str` for this call.
unsafe fn with_allowed_pos_refs<R>(
    allowed_pos: *const c_char,
    f: impl FnOnce(Option<&[&str]>) -> R,
) -> R {
    let storage = parse_allowed_pos(allowed_pos);
    let refs = storage
        .as_ref()
        .map(|v| v.iter().map(String::as_str).collect::<Vec<&str>>());

    f(refs.as_deref())
}

// ------ Tests ------ //
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

        let result = unsafe { opencc_jieba_zho_check(&opencc as *const OpenCC, input) };

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

        let result_ptr =
            unsafe { opencc_jieba_convert(&opencc as *const OpenCC, input, config, true) };
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
    fn test_opencc_jieba_convert_hong_kong_phrase_configs() {
        let opencc = OpenCC::new();

        for (input_text, config_name, expected) in
            [("鼠標", "t2hkp", "滑鼠"), ("滑鼠", "hk2tp", "鼠標")]
        {
            let input = raw_cstring(input_text);
            let config = raw_cstring(config_name);
            let result_ptr =
                unsafe { opencc_jieba_convert(&opencc as *const OpenCC, input, config, false) };
            assert!(!result_ptr.is_null());
            let result = unsafe { CString::from_raw(result_ptr).to_string_lossy().into_owned() };

            unsafe {
                reclaim_raw_cstring(config);
                reclaim_raw_cstring(input);
            }

            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_opencc_jieba_convert_2() {
        let opencc = opencc_jieba_new();
        let input =
            raw_cstring("豫章故郡，洪都新府。星分翼軫，地接衡廬。襟三江而帶五湖，控蠻荊而引甌越。");
        let config = raw_cstring("t2s");

        let result_ptr = unsafe { opencc_jieba_convert(opencc, input, config, true) };
        let result = unsafe { CString::from_raw(result_ptr).to_string_lossy().into_owned() };

        unsafe {
            reclaim_raw_cstring(config);
            reclaim_raw_cstring(input);
        }

        assert_eq!(
            result,
            "豫章故郡，洪都新府。星分翼轸，地接衡庐。襟三江而带五湖，控蛮荆而引瓯越。"
        );
        unsafe { opencc_jieba_delete(opencc) };
    }

    #[test]
    fn test_opencc_jieba_cut() {
        let opencc = OpenCC::new();
        let input = raw_cstring("你好，世界！");

        let result = unsafe { opencc_jieba_cut(&opencc as *const OpenCC, input, true) };
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

        let result =
            unsafe { opencc_jieba_cut_and_join(&opencc as *const OpenCC, input, false, delimiter) };
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
        let strings = [c1.as_ptr(), c2.as_ptr(), ptr::null()];
        let delimiter = CString::new(" ").unwrap();

        let result = unsafe { opencc_jieba_join_str(strings.as_ptr(), delimiter.as_ptr()) };
        assert!(!result.is_null());

        let result_string = unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() };
        assert_eq!(result_string, "Hello World");

        unsafe { opencc_jieba_free_string(result) };
    }

    #[test]
    fn test_opencc_jieba_keyword_extract_textrank() {
        let opencc = OpenCC::new();
        let input = CString::new(include_str!("../../../tests/data/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method = raw_cstring("textrank");

        let result = unsafe { opencc_jieba_keywords(&opencc as *const OpenCC, input, 10, method) };
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
        let input = CString::new(include_str!("../../../tests/data/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method = raw_cstring("tfidf");

        let result = unsafe { opencc_jieba_keywords(&opencc as *const OpenCC, input, 10, method) };
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
    fn test_opencc_jieba_keyword_extract_textrank_pos() {
        let opencc = OpenCC::new();
        let input = CString::new(include_str!("../../../tests/data/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method = raw_cstring("textrank");
        let allowed_pos = raw_cstring("n nr ns nt nz v vn");

        let result = unsafe {
            opencc_jieba_keywords_pos(&opencc as *const OpenCC, input, 10, method, allowed_pos)
        };
        assert!(!result.is_null());

        let out = unsafe { cstr_array_to_vec_str(result) };
        assert!(!out.is_empty());

        unsafe {
            opencc_jieba_free_string_array(result);
            reclaim_raw_cstring(input);
            reclaim_raw_cstring(method);
            reclaim_raw_cstring(allowed_pos);
        }
    }

    #[test]
    fn test_opencc_jieba_keyword_extract_tfidf_pos() {
        let opencc = OpenCC::new();
        let input = CString::new(include_str!("../../../tests/data/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method = raw_cstring("tfidf");
        let allowed_pos = raw_cstring("n nr ns nt nz v vn");

        let result = unsafe {
            opencc_jieba_keywords_pos(&opencc as *const OpenCC, input, 10, method, allowed_pos)
        };
        assert!(!result.is_null());

        let out = unsafe { cstr_array_to_vec_str(result) };
        assert!(!out.is_empty());

        unsafe {
            opencc_jieba_free_string_array(result);
            reclaim_raw_cstring(input);
            reclaim_raw_cstring(method);
            reclaim_raw_cstring(allowed_pos);
        }
    }

    #[test]
    fn test_opencc_jieba_keyword_extract_textrank_pos_empty_equivalent() {
        let opencc = OpenCC::new();
        let input = CString::new(include_str!("../../../tests/data/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method = raw_cstring("textrank");
        let allowed_pos = raw_cstring("");

        let result = unsafe {
            opencc_jieba_keywords_pos(&opencc as *const OpenCC, input, 10, method, allowed_pos)
        };
        assert!(!result.is_null());

        let out = unsafe { cstr_array_to_vec_str(result) };
        assert!(!out.is_empty());

        unsafe {
            opencc_jieba_free_string_array(result);
            reclaim_raw_cstring(input);
            reclaim_raw_cstring(method);
            reclaim_raw_cstring(allowed_pos);
        }
    }

    #[test]
    fn test_opencc_jieba_keyword_extract_textrank_pos_compare() {
        let opencc = OpenCC::new();
        let input = CString::new(include_str!("../../../tests/data/OneDay.txt"))
            .unwrap()
            .into_raw();
        let method_all = raw_cstring("textrank");
        let method_pos = raw_cstring("textrank");
        let allowed_pos = raw_cstring("n nr ns nt nz v vn");

        let result_all =
            unsafe { opencc_jieba_keywords(&opencc as *const OpenCC, input, 10, method_all) };
        assert!(!result_all.is_null());
        let out_all = unsafe { cstr_array_to_vec_str(result_all) };
        assert!(!out_all.is_empty());

        let result_pos = unsafe {
            opencc_jieba_keywords_pos(&opencc as *const OpenCC, input, 10, method_pos, allowed_pos)
        };
        assert!(!result_pos.is_null());
        let out_pos = unsafe { cstr_array_to_vec_str(result_pos) };
        assert!(!out_pos.is_empty());

        unsafe {
            opencc_jieba_free_string_array(result_all);
            opencc_jieba_free_string_array(result_pos);
            reclaim_raw_cstring(input);
            reclaim_raw_cstring(method_all);
            reclaim_raw_cstring(method_pos);
            reclaim_raw_cstring(allowed_pos);
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

        let result = unsafe {
            opencc_jieba_keywords_and_weights(
                &opencc as *const OpenCC,
                input.as_ptr(),
                5,
                method.as_ptr(),
                &mut keyword_count,
                &mut keywords,
                &mut weights,
            )
        };

        assert_eq!(result, 0);
        assert!(keyword_count > 0);

        let (keyword_vec, weight_vec) =
            collect_keywords_and_weights(keywords, weights, keyword_count);

        assert_eq!(keyword_vec.len(), keyword_count);
        assert!(weight_vec.iter().all(|w| *w >= 0.0));

        unsafe { opencc_jieba_free_keywords_and_weights(keywords, weights, keyword_count) };
    }

    #[test]
    fn test_opencc_jieba_keyword_weight_textrank_pos() {
        let opencc = OpenCC::new();
        let input = CString::new("这是一个测试文本，关键词提取演示。").unwrap();
        let method = CString::new("textrank").unwrap();
        let allowed_pos = CString::new("n nr ns nt nz v vn").unwrap();

        let mut keyword_count = 0usize;
        let mut keywords: *mut *mut c_char = ptr::null_mut();
        let mut weights: *mut f64 = ptr::null_mut();

        let result = unsafe {
            opencc_jieba_keywords_and_weights_pos(
                &opencc as *const OpenCC,
                input.as_ptr(),
                5,
                method.as_ptr(),
                allowed_pos.as_ptr(),
                &mut keyword_count,
                &mut keywords,
                &mut weights,
            )
        };

        assert_eq!(result, 0);
        assert!(keyword_count > 0);

        let (keyword_vec, weight_vec) =
            collect_keywords_and_weights(keywords, weights, keyword_count);

        assert_eq!(keyword_vec.len(), keyword_count);
        assert!(weight_vec.iter().all(|w| *w >= 0.0));

        unsafe { opencc_jieba_free_keywords_and_weights(keywords, weights, keyword_count) };
    }

    #[test]
    fn test_opencc_jieba_keyword_weight_tfidf_pos() {
        let opencc = OpenCC::new();
        let input = CString::new("这是一个测试文本，关键词提取演示。").unwrap();
        let method = CString::new("tfidf").unwrap();
        let allowed_pos = CString::new("n nr ns nt nz v vn").unwrap();

        let mut keyword_count = 0usize;
        let mut keywords: *mut *mut c_char = ptr::null_mut();
        let mut weights: *mut f64 = ptr::null_mut();

        let result = unsafe {
            opencc_jieba_keywords_and_weights_pos(
                &opencc as *const OpenCC,
                input.as_ptr(),
                5,
                method.as_ptr(),
                allowed_pos.as_ptr(),
                &mut keyword_count,
                &mut keywords,
                &mut weights,
            )
        };

        assert_eq!(result, 0);
        assert!(keyword_count > 0);

        let (keyword_vec, weight_vec) =
            collect_keywords_and_weights(keywords, weights, keyword_count);

        assert_eq!(keyword_vec.len(), keyword_count);
        assert!(weight_vec.iter().all(|w| *w >= 0.0));

        unsafe { opencc_jieba_free_keywords_and_weights(keywords, weights, keyword_count) };
    }

    fn collect_keywords_and_weights(
        keywords: *mut *mut c_char,
        weights: *mut f64,
        keyword_count: usize,
    ) -> (Vec<String>, Vec<f64>) {
        unsafe {
            let keyword_vec: Vec<String> = (0..keyword_count)
                .map(|i| {
                    CStr::from_ptr(*keywords.add(i))
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            let weight_vec: Vec<f64> = std::slice::from_raw_parts(weights, keyword_count).to_vec();

            (keyword_vec, weight_vec)
        }
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
        let array = unsafe { opencc_jieba_tag(instance, input.as_ptr(), true) };
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

        unsafe { opencc_jieba_free_tag_array(array) };
        unsafe { opencc_jieba_delete(instance) };
    }
}

#[cfg(test)]
mod compatibility_normalization_capi_tests {
    use super::*;

    fn read_owned_string(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null());
        let value = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_owned();
        unsafe { opencc_jieba_free_string(ptr) };
        value
    }

    #[test]
    fn normalize_compat_replaces_cjk_compatibility_ideographs() {
        let opencc = OpenCC::new();
        let input = CString::new("天龍八部書").unwrap();

        let result = unsafe { opencc_jieba_normalize_compat(&opencc, input.as_ptr()) };

        assert_eq!(read_owned_string(result), "天龍八部書");
    }

    #[test]
    fn normalize_compat_extended_then_t2s() {
        let opencc = OpenCC::new();
        let input = CString::new("天龍八部書裡的聼眾‧聼聼竒羙⽟䂖甁噐⾳").unwrap();

        let normalized_ptr =
            unsafe { opencc_jieba_normalize_compat_extended(&opencc, input.as_ptr()) };
        assert!(!normalized_ptr.is_null());

        let normalized = unsafe { CStr::from_ptr(normalized_ptr) }
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(normalized, "天龍八部書裡的聽眾·聽聽奇美玉石瓶器音");

        let normalized_input = CString::new(normalized).unwrap();
        let config = CString::new("t2s").unwrap();
        let simplified_ptr = unsafe {
            opencc_jieba_convert(&opencc, normalized_input.as_ptr(), config.as_ptr(), false)
        };
        assert!(!simplified_ptr.is_null());

        assert_eq!(
            read_owned_string(simplified_ptr),
            "天龙八部书里的听众·听听奇美玉石瓶器音"
        );

        unsafe { opencc_jieba_free_string(normalized_ptr) };
    }

    #[test]
    fn normalize_compat_rejects_null_argument() {
        opencc_jieba_clear_last_error();
        let input = CString::new("天龍八部").unwrap();

        let result = unsafe { opencc_jieba_normalize_compat(ptr::null(), input.as_ptr()) };

        assert!(result.is_null());
        assert_eq!(
            read_owned_string(opencc_jieba_last_error()),
            "Invalid argument: instance/input is NULL"
        );
    }

    #[test]
    fn normalize_compat_rejects_invalid_utf8() {
        opencc_jieba_clear_last_error();
        let opencc = OpenCC::new();
        let input = [0xff_u8, 0];

        let result =
            unsafe { opencc_jieba_normalize_compat(&opencc, input.as_ptr() as *const c_char) };

        assert!(result.is_null());
        assert_eq!(
            read_owned_string(opencc_jieba_last_error()),
            "Invalid UTF-8 input"
        );
    }
}

#[cfg(test)]
mod detofu_capi_tests {
    use super::*;

    fn read_owned_string(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null());
        let value = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_owned();
        unsafe { opencc_jieba_free_string(ptr) };
        value
    }

    #[test]
    fn detofu_ext_b_replaces_known_mapping() {
        let opencc = OpenCC::new();
        let input = CString::new("骖𬴂").unwrap();

        let result = unsafe { opencc_jieba_detofu(&opencc, input.as_ptr(), 0) };

        assert_eq!(read_owned_string(result), "骖騑");
    }

    #[test]
    fn detofu_all_ffi_levels_are_valid() {
        let opencc = OpenCC::new();
        let input = CString::new("普通中文").unwrap();

        for level in 0..=7 {
            let result = unsafe { opencc_jieba_detofu(&opencc, input.as_ptr(), level) };
            assert_eq!(read_owned_string(result), "普通中文");
        }
    }

    #[test]
    fn detofu_rejects_invalid_level() {
        opencc_jieba_clear_last_error();
        let opencc = OpenCC::new();
        let input = CString::new("骖𬴂").unwrap();

        let result = unsafe { opencc_jieba_detofu(&opencc, input.as_ptr(), 99) };

        assert!(result.is_null());
        assert_eq!(
            read_owned_string(opencc_jieba_last_error()),
            "Invalid DeTofu level: 99"
        );
    }

    #[test]
    fn detofu_rejects_null_and_invalid_utf8() {
        opencc_jieba_clear_last_error();
        let opencc = OpenCC::new();
        let valid = CString::new("骖𬴂").unwrap();

        assert!(unsafe { opencc_jieba_detofu(ptr::null(), valid.as_ptr(), 0) }.is_null());
        assert_eq!(
            read_owned_string(opencc_jieba_last_error()),
            "Invalid argument: instance/input is NULL"
        );

        let invalid = [0xff_u8, 0];
        assert!(
            unsafe { opencc_jieba_detofu(&opencc, invalid.as_ptr() as *const c_char, 0,) }
                .is_null()
        );
        assert_eq!(
            read_owned_string(opencc_jieba_last_error()),
            "Invalid UTF-8 input"
        );
    }
}

#[cfg(test)]
mod add_word_tests {
    use super::*;

    #[test]
    fn dynamic_words_segment_and_tag_on_same_instance() {
        unsafe {
            let instance = opencc_jieba_new();
            assert!(!instance.is_null());
            let word = CString::new("星际量子咖啡馆").unwrap();
            let tag = CString::new("nz").unwrap();
            assert!(opencc_jieba_add_word(
                instance,
                word.as_ptr(),
                true,
                100_000,
                tag.as_ptr()
            ));
            let tokens = opencc_jieba_cut(instance, word.as_ptr(), false);
            assert!(!tokens.is_null());
            assert_eq!(CStr::from_ptr(*tokens), word.as_c_str());
            assert!((*tokens.add(1)).is_null());
            opencc_jieba_free_string_array(tokens);
            let tags = opencc_jieba_tag(instance, word.as_ptr(), false);
            assert!(!tags.is_null());
            assert_eq!(CStr::from_ptr((*tags).tag), tag.as_c_str());
            opencc_jieba_free_tag_array(tags);

            // Automatic frequency ignores freq, even an otherwise extreme value.
            assert!(opencc_jieba_add_word(
                instance,
                word.as_ptr(),
                false,
                usize::MAX,
                ptr::null()
            ));
            assert!(opencc_jieba_add_word(
                instance,
                word.as_ptr(),
                true,
                0,
                ptr::null()
            ));
            let tokens = opencc_jieba_cut(instance, word.as_ptr(), false);
            assert!(!tokens.is_null());
            assert_ne!(
                CStr::from_ptr(*tokens),
                word.as_c_str(),
                "zero frequency must not become automatic"
            );
            opencc_jieba_free_string_array(tokens);
            assert!(opencc_jieba_add_word(
                instance,
                word.as_ptr(),
                true,
                100_000,
                tag.as_ptr()
            ));
            let tokens = opencc_jieba_cut(instance, word.as_ptr(), false);
            assert!(!tokens.is_null());
            assert_eq!(CStr::from_ptr(*tokens), word.as_c_str());
            opencc_jieba_free_string_array(tokens);
            opencc_jieba_delete(instance);
        }
    }

    #[test]
    fn invalid_inputs_report_errors_and_success_clears_them() {
        unsafe {
            let instance = opencc_jieba_new();
            assert!(!instance.is_null());
            let word = CString::new("星际量子咖啡馆").unwrap();
            let empty = CString::new("").unwrap();
            let space_word = CString::new("bad word").unwrap();
            let unicode_space = CString::new("bad\u{3000}word").unwrap();
            let space_tag = CString::new("n\ttag").unwrap();
            let invalid = [0xff_u8, 0];
            for (inst, w, t, expected) in [
                (ptr::null_mut(), word.as_ptr(), ptr::null(), "NULL"),
                (instance, ptr::null(), ptr::null(), "NULL"),
                (instance, empty.as_ptr(), ptr::null(), "word must not"),
                (instance, space_word.as_ptr(), ptr::null(), "word must not"),
                (
                    instance,
                    unicode_space.as_ptr(),
                    ptr::null(),
                    "word must not",
                ),
                (instance, word.as_ptr(), space_tag.as_ptr(), "tag must not"),
                (instance, invalid.as_ptr().cast(), ptr::null(), "UTF-8 word"),
                (
                    instance,
                    word.as_ptr(),
                    invalid.as_ptr().cast(),
                    "UTF-8 tag",
                ),
            ] {
                assert!(!opencc_jieba_add_word(inst, w, false, 0, t));
                let error = opencc_jieba_last_error();
                assert!(!error.is_null());
                assert!(CStr::from_ptr(error).to_str().unwrap().contains(expected));
                opencc_jieba_free_string(error);
                assert!(get_c_api_last_error().is_some());
                assert!(opencc_jieba_add_word(
                    instance,
                    word.as_ptr(),
                    false,
                    0,
                    ptr::null()
                ));
                assert!(get_c_api_last_error().is_none());
            }
            // A non-NULL empty tag is accepted as Some("").
            assert!(opencc_jieba_add_word(
                instance,
                word.as_ptr(),
                true,
                100_000,
                empty.as_ptr()
            ));
            assert_eq!(
                CStr::from_ptr(opencc_jieba_version_string())
                    .to_str()
                    .unwrap(),
                "0.8.1.1"
            );
            assert_eq!(opencc_jieba_abi_number(), 1);
            opencc_jieba_delete(instance);
        }
    }
}

#[cfg(test)]
mod constructor_tests {
    use super::*;

    #[test]
    fn user_dict_constructor_accepts_null_zero() {
        let instance = unsafe { opencc_jieba_new_user_dict(ptr::null(), 0) };
        assert!(!instance.is_null());
        unsafe { opencc_jieba_delete(instance) };
    }

    #[test]
    fn custom_constructor_accepts_null_zero() {
        let instance = unsafe { opencc_jieba_new_custom(ptr::null(), 0) };
        assert!(!instance.is_null());
        unsafe { opencc_jieba_delete(instance) };
    }

    #[test]
    fn combined_constructor_applies_user_entry_and_custom_mapping() {
        let word = CString::new("帕兰蒂尔").unwrap();
        let user_entry = OpenccJiebaUserDictEntry {
            word: word.as_ptr(),
            freq: 100_000,
            tag: ptr::null(),
        };

        let source = CString::new("帕兰蒂尔").unwrap();
        let target = CString::new("柏蘭蒂爾").unwrap();
        let pair = OpenccJiebaCustomPair {
            source: source.as_ptr(),
            target: target.as_ptr(),
        };
        let spec = OpenccJiebaCustomDictSpec {
            slot: 2,
            mode: OPENCC_JIEBA_CUSTOM_DICT_APPEND,
            pairs: &pair,
            pair_count: 1,
        };

        let instance = unsafe { opencc_jieba_new_user_dict_custom(&user_entry, 1, &spec, 1) };
        assert!(!instance.is_null());

        let input = CString::new("帕兰蒂尔").unwrap();
        let config = CString::new("s2t").unwrap();
        let output =
            unsafe { opencc_jieba_convert(instance, input.as_ptr(), config.as_ptr(), false) };

        assert!(!output.is_null());
        unsafe {
            assert_eq!(CStr::from_ptr(output).to_str().unwrap(), "柏蘭蒂爾");
        }

        unsafe { opencc_jieba_free_string(output) };
        unsafe { opencc_jieba_delete(instance) };
    }

    #[test]
    fn constructor_reports_invalid_null_nonempty_array() {
        let instance = unsafe { opencc_jieba_new_user_dict(ptr::null(), 1) };
        assert!(instance.is_null());

        let error = opencc_jieba_last_error();
        assert!(!error.is_null());

        unsafe {
            assert_eq!(
                CStr::from_ptr(error).to_str().unwrap(),
                "Invalid argument: entries is NULL while entry_count is 1"
            );
        }

        unsafe { opencc_jieba_free_string(error) };
    }
}
