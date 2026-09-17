#ifndef FINKIT_RESEARCH_HPP
#define FINKIT_RESEARCH_HPP

#include "finkit_research.h"
#include <stdexcept>
#include <string>

namespace finkit {
namespace research {

inline std::string take_json_response(char *raw, const char *operation) {
    if (raw == nullptr) {
        throw std::runtime_error(std::string("finkit ") + operation + " returned a null response");
    }
    std::string response(raw);
    finkit_factor_study_free_string(raw);
    return response;
}

inline std::string factor_study_json(const std::string &request_json) {
    return take_json_response(finkit_factor_study_json(request_json.c_str()), "factor study");
}

inline std::string quant_evaluation_json(const std::string &request_json) {
    return take_json_response(
        finkit_quant_evaluation_json(request_json.c_str()),
        "quantitative evaluation");
}

} // namespace research
} // namespace finkit

#endif /* FINKIT_RESEARCH_HPP */