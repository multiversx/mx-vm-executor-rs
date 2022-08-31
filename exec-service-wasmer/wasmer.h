
#if !defined(WASMER_H_MACROS)

#define WASMER_H_MACROS

// Define the `ARCH_X86_X64` constant.
#if defined(MSVC) && defined(_M_AMD64)
#  define ARCH_X86_64
#elif (defined(GCC) || defined(__GNUC__) || defined(__clang__)) && defined(__x86_64__)
#  define ARCH_X86_64
#endif

// Compatibility with non-Clang compilers.
#if !defined(__has_attribute)
#  define __has_attribute(x) 0
#endif

// Compatibility with non-Clang compilers.
#if !defined(__has_declspec_attribute)
#  define __has_declspec_attribute(x) 0
#endif

// Define the `DEPRECATED` macro.
#if defined(GCC) || defined(__GNUC__) || __has_attribute(deprecated)
#  define DEPRECATED(message) __attribute__((deprecated(message)))
#elif defined(MSVC) || __has_declspec_attribute(deprecated)
#  define DEPRECATED(message) __declspec(deprecated(message))
#endif

#endif // WASMER_H_MACROS


#ifndef WASMER_H
#define WASMER_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * The `wasmer_result_t` enum is a type that represents either a
 * success, or a failure.
 */
typedef enum {
  /**
   * Represents a success.
   */
  WASMER_OK = 1,
  /**
   * Represents a failure.
   */
  WASMER_ERROR = 2,
} wasmer_result_t;

typedef struct {

} wasmer_import_func_t;

typedef struct {
  const uint8_t *bytes;
  uint32_t bytes_len;
} wasmer_byte_array;

typedef struct {
  wasmer_byte_array module_name;
  wasmer_byte_array import_name;
  wasmer_import_export_kind tag;
  wasmer_import_export_value value;
} wasmer_import_t;

/**
 * Frees memory for the given Func
 */
void wasmer_import_func_destroy(wasmer_import_func_t *func);

/**
 * Creates new host function, aka imported function. `func` is a
 * function pointer, where the first argument is the famous `vm::Ctx`
 * (in Rust), or `wasmer_instance_context_t` (in C). All arguments
 * must be typed with compatible WebAssembly native types:
 *
 * | WebAssembly type | C/C++ type |
 * | ---------------- | ---------- |
 * | `i32`            | `int32_t`  |
 * | `i64`            | `int64_t`  |
 * | `f32`            | `float`    |
 * | `f64`            | `double`   |
 *
 * The function pointer must have a lifetime greater than the
 * WebAssembly instance lifetime.
 *
 * The caller owns the object and should call
 * `wasmer_import_func_destroy` to free it.
 */
wasmer_import_func_t *wasmer_import_func_new(void (*func)(void *data),
                                             const wasmer_value_tag *params,
                                             unsigned int params_len,
                                             const wasmer_value_tag *returns,
                                             unsigned int returns_len);

wasmer_result_t wasmer_import_object_cache_from_imports(wasmer_import_t *imports,
                                                        unsigned int imports_len);

#endif /* WASMER_H */
