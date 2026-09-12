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

/** Versioned JSON-in/JSON-out factor research API.
 * Current response schema_version=2; request schema versions 1 and 2 are accepted.
 * The returned UTF-8 string must be released by
 * finkit_factor_study_free_string().
 */
FINKIT_RESEARCH_API char *finkit_factor_study_json(const char *request_json);

/** Generic strategy/portfolio quantitative evaluation API (schema_version=1).
 * Supports return, drawdown, risk-adjusted, benchmark, trade, portfolio and
 * after-cost metrics. The returned UTF-8 string uses the same release function.
 */
FINKIT_RESEARCH_API char *finkit_quant_evaluation_json(const char *request_json);

/** Release strings returned by either JSON API above. */
FINKIT_RESEARCH_API void finkit_factor_study_free_string(char *value);

#ifdef __cplusplus
}
#endif

#endif /* FINKIT_RESEARCH_H */