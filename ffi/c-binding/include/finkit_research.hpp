#ifndef FINKIT_RESEARCH_HPP
#define FINKIT_RESEARCH_HPP

#include "finkit_research.h"
#include <stdexcept>
#include <string>

namespace finkit {
namespace research {

inline std::string factor_study_json(const std::string &request_json) {
    char *raw = finkit_factor_study_json(request_json.c_str());
    if (raw == nullptr) {
        throw std::runtime_error("finkit factor study returned a null response");
    }
    std::string response(raw);
    finkit_factor_study_free_string(raw);
    return response;
}

} // namespace research
} // namespace finkit

#endif /* FINKIT_RESEARCH_HPP */
