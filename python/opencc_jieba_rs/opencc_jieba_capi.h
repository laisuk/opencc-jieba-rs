#ifndef OPENCC_JIEBA_CAPI_H
#define OPENCC_JIEBA_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>

void *opencc_new();
char *opencc_convert(const void *instance, const char *input, const char *config, bool punctuation);
int opencc_zho_check(const void *instance, const char *input);
void opencc_free(const void *instance);
void opencc_string_free(const char *ptr);
char **opencc_jieba_cut(const void *instance, const char *input, bool hmm);
void opencc_free_string_array(char **array);
char *opencc_join_str(char **strings, const char *delimiter);
char *opencc_jieba_cut_and_join(const void *instance, const char *input, bool hmm, const char *delimiter);
char **opencc_jieba_keyword_extract_textrank(const void *instance, const char *input, int top_k);
char **opencc_jieba_keyword_extract_tfidf(const void *instance, const char *input, int top_k);

#ifdef __cplusplus
}
#endif

#endif // OPENCC_JIEBA_CAPI_H
