package ta

/*
#cgo LDFLAGS: -lfinkit_go
#include <stdlib.h>
char* finkit_go_factor_study_json(const char* request_json);
void finkit_go_factor_study_free_string(char* value);
*/
import "C"
import (
	"errors"
	"unsafe"
)

// FactorStudyJSON runs the schema-versioned canonical Rust factor research engine.
func FactorStudyJSON(requestJSON string) (string, error) {
	request := C.CString(requestJSON)
	defer C.free(unsafe.Pointer(request))
	response := C.finkit_go_factor_study_json(request)
	if response == nil {
		return "", errors.New("native factor research returned nil")
	}
	defer C.finkit_go_factor_study_free_string(response)
	return C.GoString(response), nil
}
