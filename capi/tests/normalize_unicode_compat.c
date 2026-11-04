#include "opencc_jieba_capi.h"
#include <assert.h>
#include <string.h>

int main(void) {
    void *instance = opencc_jieba_new();
    assert(instance);
    const char *input = "\xe8\x81\xbc\xe2\xbd\x9f\xef\xa8\x99"; /* 聼⽟神 */
    char *output = opencc_jieba_normalize_unicode_compat(instance, input);
    assert(output && strcmp(output, "\xe8\x81\xbd\xe7\x8e\x89\xef\xa8\x99") == 0);
    opencc_jieba_free_string(output);
    assert(!opencc_jieba_normalize_unicode_compat(NULL, input));
    assert(!opencc_jieba_normalize_unicode_compat(instance, NULL));
    const char invalid[] = {(char)0xff, 0};
    assert(!opencc_jieba_normalize_unicode_compat(instance, invalid));
    char *error = opencc_jieba_last_error();
    assert(error && strcmp(error, "Invalid UTF-8 input") == 0);
    opencc_jieba_free_string(error);
    output = opencc_jieba_normalize_unicode_compat(instance, "");
    assert(output && output[0] == '\0');
    opencc_jieba_free_string(output);
    error = opencc_jieba_last_error();
    assert(error && strcmp(error, "No error") == 0);
    opencc_jieba_free_string(error);
    opencc_jieba_delete(instance);
}
