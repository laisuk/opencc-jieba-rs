#include "opencc_jieba_capi.h"

#include <assert.h>
#include <string.h>

int main(void) {
    bool (*add_word)(void *, const char *, bool, size_t, const char *) = opencc_jieba_add_word;
    void *instance = opencc_jieba_new();
    assert(instance != NULL);
    assert(add_word(instance, "dynamiccapiword", true, 100000, NULL));
    char **tokens = opencc_jieba_cut(instance, "dynamiccapiword", false);
    assert(tokens != NULL && tokens[0] != NULL);
    assert(strcmp(tokens[0], "dynamiccapiword") == 0 && tokens[1] == NULL);
    opencc_jieba_free_string_array(tokens);
    assert(strcmp(opencc_jieba_version_string(), "0.8.1.1") == 0);
    opencc_jieba_delete(instance);
    return 0;
}
