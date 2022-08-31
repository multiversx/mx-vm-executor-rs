package main

// NOTE: There should be NO space between the comments and the `import "C"` line.

/*
#cgo LDFLAGS: -Wl,-rpath,${SRCDIR} -L${SRCDIR} -lcapiprototype
#include "./capiprototype.h"
*/
import "C"

func main() {
	C.init_stuff()
	// C.hello(C.CString("John Smith"))
	// C.ew_run_simple()
}
