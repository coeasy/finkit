#include <stdio.h>
#include <string.h>

#include "finkit.h"

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
    return 0;
}
