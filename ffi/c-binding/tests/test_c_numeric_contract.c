#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "finkit.h"
#include "talib_numeric_contract_generated.h"

typedef struct NumericValue {
    int is_null;
    double value;
} NumericValue;

typedef struct NumericOutput {
    char *name;
    NumericValue *values;
    size_t length;
} NumericOutput;

static const char *skip_space(const char *cursor) {
    while (*cursor == ' ' || *cursor == '\n' || *cursor == '\r' || *cursor == '\t') {
        ++cursor;
    }
    return cursor;
}

static int append_value(NumericValue **values, size_t *length, NumericValue value) {
    NumericValue *next = (NumericValue *)realloc(
        *values, (*length + 1) * sizeof(NumericValue));
    if (next == NULL) {
        return 0;
    }
    next[*length] = value;
    *values = next;
    ++*length;
    return 1;
}

static int parse_array(const char **cursor, NumericValue **values, size_t *length) {
    const char *current = skip_space(*cursor);
    if (*current != '[') {
        return 0;
    }
    ++current;
    current = skip_space(current);
    if (*current == ']') {
        *cursor = current + 1;
        return 1;
    }
    for (;;) {
        NumericValue value;
        current = skip_space(current);
        if (strncmp(current, "null", 4) == 0) {
            value.is_null = 1;
            value.value = 0.0;
            current += 4;
        } else {
            char *end = NULL;
            value.value = strtod(current, &end);
            if (end == current) {
                return 0;
            }
            value.is_null = 0;
            current = end;
        }
        if (!append_value(values, length, value)) {
            return 0;
        }
        current = skip_space(current);
        if (*current == ']') {
            *cursor = current + 1;
            return 1;
        }
        if (*current != ',') {
            return 0;
        }
        ++current;
    }
}

static void free_outputs(NumericOutput *outputs, size_t count) {
    size_t index;
    for (index = 0; index < count; ++index) {
        free(outputs[index].name);
        free(outputs[index].values);
    }
    free(outputs);
}

static int append_output(NumericOutput **outputs, size_t *count, NumericOutput output) {
    NumericOutput *next = (NumericOutput *)realloc(
        *outputs, (*count + 1) * sizeof(NumericOutput));
    if (next == NULL) {
        return 0;
    }
    next[*count] = output;
    *outputs = next;
    ++*count;
    return 1;
}

static int parse_object_arrays(const char *json, NumericOutput **outputs, size_t *count) {
    const char *current = skip_space(json);
    *outputs = NULL;
    *count = 0;
    if (*current != '{') {
        return 0;
    }
    ++current;
    current = skip_space(current);
    if (*current == '}') {
        return 1;
    }
    for (;;) {
        const char *key_start;
        const char *key_end;
        NumericOutput output;
        current = skip_space(current);
        if (*current != '"') {
            free_outputs(*outputs, *count);
            *outputs = NULL;
            *count = 0;
            return 0;
        }
        key_start = ++current;
        key_end = strchr(key_start, '"');
        if (key_end == NULL) {
            free_outputs(*outputs, *count);
            *outputs = NULL;
            *count = 0;
            return 0;
        }
        output.name = (char *)malloc((size_t)(key_end - key_start) + 1);
        if (output.name == NULL) {
            free_outputs(*outputs, *count);
            *outputs = NULL;
            *count = 0;
            return 0;
        }
        memcpy(output.name, key_start, (size_t)(key_end - key_start));
        output.name[key_end - key_start] = '\0';
        current = skip_space(key_end + 1);
        if (*current != ':') {
            free(output.name);
            free_outputs(*outputs, *count);
            *outputs = NULL;
            *count = 0;
            return 0;
        }
        ++current;
        output.values = NULL;
        output.length = 0;
        if (!parse_array(&current, &output.values, &output.length) ||
            !append_output(outputs, count, output)) {
            free(output.name);
            free(output.values);
            free_outputs(*outputs, *count);
            *outputs = NULL;
            *count = 0;
            return 0;
        }
        current = skip_space(current);
        if (*current == '}') {
            return 1;
        }
        if (*current != ',') {
            free_outputs(*outputs, *count);
            *outputs = NULL;
            *count = 0;
            return 0;
        }
        ++current;
    }
}

static const NumericOutput *find_output(const NumericOutput *outputs, size_t count,
                                        const char *name) {
    size_t index;
    for (index = 0; index < count; ++index) {
        if (strcmp(outputs[index].name, name) == 0) {
            return &outputs[index];
        }
    }
    return NULL;
}

static char *make_request(const FinkitNumericContractVector *vector) {
    const size_t required = strlen(vector->operation) +
        strlen(finkit_test_contract_semantic_profile) +
        strlen(vector->input_order_json) + strlen(finkit_test_contract_inputs_json) +
        strlen(vector->params_json) + 96;
    char *request = (char *)malloc(required);
    if (request == NULL) {
        return NULL;
    }
    snprintf(request, required,
             "{\"operation\":%s,\"semantic_profile\":%s,"
             "\"input_order\":%s,\"inputs\":%s,\"params\":%s}",
             vector->operation, finkit_test_contract_semantic_profile,
             vector->input_order_json, finkit_test_contract_inputs_json,
             vector->params_json);
    return request;
}

static int check_vector(const FinkitNumericContractVector *vector) {
    char *request = make_request(vector);
    char *response;
    const char *values_marker;
    const char *values_json;
    NumericOutput *actual = NULL;
    NumericOutput *expected = NULL;
    size_t actual_count = 0;
    size_t expected_count = 0;
    size_t output_index;
    int ok = 0;

    if (request == NULL) {
        fprintf(stderr, "::error title=C numeric contract::%s: failed to allocate request\n",
                vector->operation);
        return 0;
    }
    response = ta_operation_execute_json(request);
    free(request);
    if (response == NULL) {
        fprintf(stderr, "::error title=C numeric contract::%s: null response\n",
                vector->operation);
        return 0;
    }
    if (strstr(response, "\"error\"") != NULL) {
        fprintf(stderr, "::error title=C numeric contract::%s: operation returned an error\n",
                vector->operation);
        finkit_free_string(response);
        return 0;
    }
    values_marker = strstr(response, "\"values\":");
    if (values_marker == NULL) {
        fprintf(stderr, "::error title=C numeric contract::%s: missing values envelope\n",
                vector->operation);
        finkit_free_string(response);
        return 0;
    }
    values_json = strchr(values_marker, '{');
    if (values_json == NULL ||
        !parse_object_arrays(values_json, &actual, &actual_count) ||
        !parse_object_arrays(vector->expected_json, &expected, &expected_count) ||
        actual_count != expected_count) {
        fprintf(stderr, "::error title=C numeric contract::%s: invalid output JSON or output count mismatch\n",
                vector->operation);
        free_outputs(actual, actual_count);
        free_outputs(expected, expected_count);
        finkit_free_string(response);
        return 0;
    }

    ok = 1;
    for (output_index = 0; output_index < expected_count && ok; ++output_index) {
        const NumericOutput *got = find_output(
            actual, actual_count, expected[output_index].name);
        size_t point;
        if (got == NULL || got->length != expected[output_index].length) {
            fprintf(stderr,
                    "::error title=C numeric contract::%s: missing or length-mismatched output %s\n",
                    vector->operation, expected[output_index].name);
            ok = 0;
            break;
        }
        for (point = 0; point < got->length; ++point) {
            const NumericValue want = expected[output_index].values[point];
            const NumericValue value = got->values[point];
            const double limit = vector->atol + vector->rtol * fabs(want.value);
            if (want.is_null != value.is_null ||
                (!want.is_null && fabs(value.value - want.value) > limit)) {
                fprintf(stderr,
                        "::error title=C numeric contract::%s/%s[%zu]: expected %s%.17g got %s%.17g\n",
                        vector->operation, expected[output_index].name, point,
                        want.is_null ? "null" : "", want.value,
                        value.is_null ? "null" : "", value.value);
                ok = 0;
                break;
            }
        }
    }
    free_outputs(actual, actual_count);
    free_outputs(expected, expected_count);
    finkit_free_string(response);
    return ok;
}

int main(void) {
    size_t index;
    if (finkit_test_contract_vector_count != 201) {
        fprintf(stderr, "expected 201 numeric contract vectors\n");
        return 1;
    }
    for (index = 0; index < finkit_test_contract_vector_count; ++index) {
        if (!check_vector(&finkit_test_contract_vectors[index])) {
            fprintf(stderr, "TA-Lib C numeric contract failed at vector %zu (%s)\n",
                    index, finkit_test_contract_vectors[index].operation);
            return 1;
        }
    }
    return 0;
}
