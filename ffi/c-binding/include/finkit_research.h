#ifndef FINKIT_RESEARCH_H
#define FINKIT_RESEARCH_H

#ifdef __cplusplus
extern "C" {
#endif

#ifdef _WIN32
  #ifdef FINKIT_EXPORTS
    #define FINKIT_RESEARCH_API __declspec(dllexport)
  #else
    #define FINKIT_RESEARCH_API __declspec(dllimport)
  #endif
#else
  #define FINKIT_RESEARCH_API __attribute__((visibility("default")))
#endif

/** Versioned JSON-in/JSON-out factor research API (schema_version=1).
 * The returned UTF-8 string must be released by
 * finkit_factor_study_free_string().
 */
FINKIT_RESEARCH_API char *finkit_factor_study_json(const char *request_json);
FINKIT_RESEARCH_API void finkit_factor_study_free_string(char *value);

#ifdef __cplusplus
}
#endif

#endif /* FINKIT_RESEARCH_H */
