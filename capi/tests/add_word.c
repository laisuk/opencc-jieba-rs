#include "opencc_jieba_capi.h"

#include <assert.h>
#include <string.h>

static void check_version(void) {
    const char *version = opencc_jieba_version_string();
    assert(version != NULL);
    /* Release versions have four numeric components, including revision zero. */
    for (size_t component = 0; component < 4; ++component) {
        size_t digits = strspn(version, "0123456789");
        assert(digits > 0);
        version += digits;
        if (component < 3) {
            assert(*version == '.');
            ++version;
        } else {
            assert(*version == '\0');
        }
    }
    assert(opencc_jieba_abi_number() == 1);
}

int main(void) {
    bool (*add_word)(void *, const char *, bool, size_t, const char *) = opencc_jieba_add_word;
    void *instance = opencc_jieba_new();
    assert(instance != NULL);
    assert(add_word(instance, "dynamiccapiword", true, 100000, NULL));
    char **tokens = opencc_jieba_cut(instance, "dynamiccapiword", false);
    assert(tokens != NULL && tokens[0] != NULL);
    assert(strcmp(tokens[0], "dynamiccapiword") == 0 && tokens[1] == NULL);
    opencc_jieba_free_string_array(tokens);
    check_version();
    opencc_jieba_delete(instance);
    return 0;
}
