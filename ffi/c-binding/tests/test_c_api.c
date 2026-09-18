#include <stdio.h>
#include <string.h>

#include "finkit.h"

static size_t count_occurrences(const char *text, const char *needle) {
    size_t count = 0;
    const size_t needle_length = strlen(needle);
    while ((text = strstr(text, needle)) != NULL) {
        ++count;
        text += needle_length;
    }
    return count;
}

int main(void) {
    const char *request =
        "{\"operation\":\"SMA\",\"input_order\":[\"CLOSE\"],"
        "\"inputs\":{\"CLOSE\":[1.0,2.0,3.0,4.0]},\"params\":[2.0]}";
    char *result = ta_operation_execute_json(request);
    if (result == NULL) {
        fprintf(stderr, "C API returned a null operation result\n");
        return 1;
    }
    const int ok = strstr(result, "\"operation\":\"SMA\"") != NULL &&
                   strstr(result, "\"SMA\":[1.0,1.5,2.25,3.125]") != NULL;
    finkit_free_string(result);
    if (!ok) {
        fprintf(stderr, "C API result did not match the shared engine vector\n");
        return 1;
    }

    char *catalog = ta_operation_catalog_json();
    if (catalog == NULL) {
        fprintf(stderr, "C API returned a null operation catalog\n");
        return 1;
    }
    const int catalog_ok = strstr(catalog, "\"schema_version\":1") != NULL &&
                           strstr(catalog, "\"name\":\"SMA\"") != NULL &&
                           strstr(catalog, "talib_0_8_0") != NULL &&
                           strstr(catalog, "talib_0_7_1") == NULL &&
                           count_occurrences(catalog, "\"profile_output_contracts\":{\"talib_0_8_0\"") == 201;
    finkit_free_string(catalog);
    if (!catalog_ok) {
        fprintf(stderr, "C API catalog did not publish the current 201-name TA-Lib profile\n");
        return 1;
    }
    return 0;
}
