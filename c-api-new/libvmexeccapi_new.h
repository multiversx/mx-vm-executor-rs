#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * The value of `vm_exec2_hook_result_t::early_exit` that means "the hook returned normally".
 *
 * A dedicated sentinel is needed because `0` is a real early-exit code: the Rust VM uses
 * `ASYNC_CALL_EARLY_EXIT_CODE = 0` and the Go VM has the same async-call case, so "code 0"
 * and "no code" have to stay distinguishable.
 */
#define VM_EXEC2_NO_EARLY_EXIT UINT64_MAX

/**
 * Result of a C API call.
 *
 * `VM_EXEC2_OK` is zero and every failure is negative, so the functions that return a
 * count (the memory loads) can share the same code space: a non-negative result is a
 * count, a negative one is one of these.
 */
typedef enum vm_exec2_status_t {
  /**
   * The call succeeded.
   */
  VM_EXEC2_OK = 0,
  /**
   * The call failed; the error message was written to the provided error buffer.
   */
  VM_EXEC2_ERROR = -1,
  /**
   * A pointer argument that must not be null was null. No message is produced, because
   * a null pointer may well be the error buffer itself.
   */
  VM_EXEC2_NULL_POINTER = -2,
  /**
   * A hook context was used outside the host call it belongs to. See
   * [`crate::capi_hook_context`].
   */
  VM_EXEC2_STALE_CONTEXT = -3,
  /**
   * A panic was caught at the boundary. Rust panics must never unwind into C, so every
   * entry point traps them and reports this instead.
   */
  VM_EXEC2_PANIC = -4,
} vm_exec2_status_t;

/**
 * How a call ended.
 *
 * This replaces the legacy arrangement, where a call returned a bare ok/error and the VM
 * then polled a wasm global to find out what had actually happened.
 */
typedef enum vm_exec2_call_result_kind_t {
  /**
   * The function returned normally.
   */
  VM_EXEC2_CALL_OK = 0,
  /**
   * No exported function by that name.
   */
  VM_EXEC2_CALL_FUNCTION_NOT_FOUND = 1,
  /**
   * The instance trapped for a reason that is neither a VM hook's doing nor a
   * middleware breakpoint. The message is in the error buffer.
   */
  VM_EXEC2_CALL_RUNTIME_ERROR = 2,
  /**
   * A VM hook asked for execution to stop. `early_exit_code` says which; the message, if
   * any, is in the error buffer, and is informational — the VM has already recorded its
   * own.
   */
  VM_EXEC2_CALL_EARLY_EXIT = 3,
  /**
   * A middleware stopped execution. `breakpoint` says which.
   */
  VM_EXEC2_CALL_BREAKPOINT = 4,
} vm_exec2_call_result_kind_t;

/**
 * Opaque handle to an executor.
 */
typedef struct vm_exec2_executor_t {

} vm_exec2_executor_t;

/**
 * FFI-safe mirror of `OpcodeCost`.
 *
 * This is a distinct type rather than a cast of `OpcodeCost` itself, so that
 * the general-purpose struct is free to change shape without touching the C
 * ABI, and vice versa.
 */
typedef struct vm_exec_opcode_cost_t {
  uint32_t opcode_block;
  uint32_t opcode_br;
  uint32_t opcode_brif;
  uint32_t opcode_brtable;
  uint32_t opcode_call;
  uint32_t opcode_callindirect;
  uint32_t opcode_catch;
  uint32_t opcode_catchall;
  uint32_t opcode_delegate;
  uint32_t opcode_drop;
  uint32_t opcode_else;
  uint32_t opcode_end;
  uint32_t opcode_globalget;
  uint32_t opcode_globalset;
  uint32_t opcode_i32add;
  uint32_t opcode_i32and;
  uint32_t opcode_i32clz;
  uint32_t opcode_i32const;
  uint32_t opcode_i32ctz;
  uint32_t opcode_i32divs;
  uint32_t opcode_i32divu;
  uint32_t opcode_i32eq;
  uint32_t opcode_i32eqz;
  uint32_t opcode_i32extend16s;
  uint32_t opcode_i32extend8s;
  uint32_t opcode_i32ges;
  uint32_t opcode_i32geu;
  uint32_t opcode_i32gts;
  uint32_t opcode_i32gtu;
  uint32_t opcode_i32les;
  uint32_t opcode_i32leu;
  uint32_t opcode_i32load;
  uint32_t opcode_i32load16s;
  uint32_t opcode_i32load16u;
  uint32_t opcode_i32load8s;
  uint32_t opcode_i32load8u;
  uint32_t opcode_i32lts;
  uint32_t opcode_i32ltu;
  uint32_t opcode_i32mul;
  uint32_t opcode_i32ne;
  uint32_t opcode_i32or;
  uint32_t opcode_i32popcnt;
  uint32_t opcode_i32rems;
  uint32_t opcode_i32remu;
  uint32_t opcode_i32rotl;
  uint32_t opcode_i32rotr;
  uint32_t opcode_i32shl;
  uint32_t opcode_i32shrs;
  uint32_t opcode_i32shru;
  uint32_t opcode_i32store;
  uint32_t opcode_i32store16;
  uint32_t opcode_i32store8;
  uint32_t opcode_i32sub;
  uint32_t opcode_i32wrapi64;
  uint32_t opcode_i32xor;
  uint32_t opcode_i64add;
  uint32_t opcode_i64and;
  uint32_t opcode_i64clz;
  uint32_t opcode_i64const;
  uint32_t opcode_i64ctz;
  uint32_t opcode_i64divs;
  uint32_t opcode_i64divu;
  uint32_t opcode_i64eq;
  uint32_t opcode_i64eqz;
  uint32_t opcode_i64extend16s;
  uint32_t opcode_i64extend32s;
  uint32_t opcode_i64extend8s;
  uint32_t opcode_i64extendi32s;
  uint32_t opcode_i64extendi32u;
  uint32_t opcode_i64ges;
  uint32_t opcode_i64geu;
  uint32_t opcode_i64gts;
  uint32_t opcode_i64gtu;
  uint32_t opcode_i64les;
  uint32_t opcode_i64leu;
  uint32_t opcode_i64load;
  uint32_t opcode_i64load16s;
  uint32_t opcode_i64load16u;
  uint32_t opcode_i64load32s;
  uint32_t opcode_i64load32u;
  uint32_t opcode_i64load8s;
  uint32_t opcode_i64load8u;
  uint32_t opcode_i64lts;
  uint32_t opcode_i64ltu;
  uint32_t opcode_i64mul;
  uint32_t opcode_i64ne;
  uint32_t opcode_i64or;
  uint32_t opcode_i64popcnt;
  uint32_t opcode_i64rems;
  uint32_t opcode_i64remu;
  uint32_t opcode_i64rotl;
  uint32_t opcode_i64rotr;
  uint32_t opcode_i64shl;
  uint32_t opcode_i64shrs;
  uint32_t opcode_i64shru;
  uint32_t opcode_i64store;
  uint32_t opcode_i64store16;
  uint32_t opcode_i64store32;
  uint32_t opcode_i64store8;
  uint32_t opcode_i64sub;
  uint32_t opcode_i64xor;
  uint32_t opcode_if;
  uint32_t opcode_localget;
  uint32_t opcode_localset;
  uint32_t opcode_localtee;
  uint32_t opcode_loop;
  uint32_t opcode_memorygrow;
  uint32_t opcode_memorysize;
  uint32_t opcode_nop;
  uint32_t opcode_reffunc;
  uint32_t opcode_refisnull;
  uint32_t opcode_refnull;
  uint32_t opcode_rethrow;
  uint32_t opcode_return;
  uint32_t opcode_returncall;
  uint32_t opcode_returncallindirect;
  uint32_t opcode_select;
  uint32_t opcode_tableget;
  uint32_t opcode_tablegrow;
  uint32_t opcode_tableinit;
  uint32_t opcode_tableset;
  uint32_t opcode_tablesize;
  uint32_t opcode_throw;
  uint32_t opcode_try;
  uint32_t opcode_typedselect;
  uint32_t opcode_unreachable;
  uint32_t opcode_unwind;
  uint32_t opcode_memorycopy;
  uint32_t opcode_memoryfill;
  uint32_t opcode_memorycopyperbyte;
  uint32_t opcode_memoryfillperbyte;
  uint32_t opcode_localallocate;
} vm_exec_opcode_cost_t;

/**
 * Everything an executor is configured with.
 *
 * The opcode cost table is referenced rather than inlined, because it is large and the
 * caller already holds one. It is read during the call and not retained.
 */
typedef struct vm_exec2_executor_config_t {
  /**
   * Which opcode set and whitelist to apply; see `OpcodeVersion` on the Rust side.
   */
  int32_t opcode_version;
  /**
   * The per-opcode gas costs. Must not be null.
   */
  const struct vm_exec_opcode_cost_t *opcode_cost;
} vm_exec2_executor_config_t;

/**
 * Opaque handle to an instance.
 */
typedef struct vm_exec2_instance_t {

} vm_exec2_instance_t;

/**
 * FFI-safe mirror of the new `CompilationOptions`.
 *
 * Four fields, against the legacy seven: `gas_limit` moved to the call, and metering and
 * runtime breakpoints are no longer optional. Fields that are `usize` on the Rust side are
 * fixed to `u64` here, since `usize` has no guaranteed width across host platforms.
 */
typedef struct vm_exec2_compilation_options_t {
  uint64_t unmetered_locals;
  uint64_t max_memory_grow;
  uint64_t max_memory_grow_delta;
  bool opcode_trace;
} vm_exec2_compilation_options_t;

/**
 * The out-struct of `vm_exec2_instance_call`.
 */
typedef struct vm_exec2_call_result_t {
  enum vm_exec2_call_result_kind_t kind;
  /**
   * Meaningful only for `VM_EXEC2_CALL_EARLY_EXIT`.
   */
  uint64_t early_exit_code;
  /**
   * Meaningful only for `VM_EXEC2_CALL_BREAKPOINT`. Values match `BreakpointValue`:
   * 4 = out of gas, 5 = memory limit.
   */
  uint64_t breakpoint;
} vm_exec2_call_result_t;

/**
 * What a VM hook is handed as its first argument.
 *
 * Only `go_vm_hooks` is part of the layout the Go side mirrors; the rest is private to
 * Rust, and the whole pointer is what travels back into the state functions.
 */
typedef struct vm_exec2_hook_context_t {
  /**
   * The Go VM hooks object for this instance. Read by the Go side, never by Rust.
   */
  void *go_vm_hooks;
  /**
   * Erased `&mut ExperimentalInstanceState<'_>`; null while no call is active.
   */
  void *state;
  /**
   * Erased `&RefCell<CapiHookContextPool>`, for the token check.
   */
  const void *pool;
  /**
   * This activation's call token; `0` while no call is active.
   */
  uint64_t token;
} vm_exec2_hook_context_t;

/**
 * What every VM hook returns.
 *
 * One 16-byte struct covers every hook shape — `void`, `i32`, `i64` — and is returned in
 * registers under both SysV AMD64 (RAX:RDX) and AAPCS64 (X0:X1), so the early-exit channel
 * costs no memory traffic. A cgo `//export` function may return a C struct by value, which
 * is what makes this shape available to the Go side.
 *
 * The early-exit *message* does not travel in this struct. A pointer cannot: cgo forbids
 * returning an unpinned Go pointer, and a fixed buffer cannot work because the messages are
 * unbounded user text. The Go VM does not need it to travel, because the hook has already
 * recorded the message in its own output state by the time it returns.
 */
typedef struct vm_exec2_hook_result_t {
  /**
   * The hook's own return value, widened to 64 bits. Ignored for a `void` hook, and for
   * any hook that reports an early exit.
   */
  int64_t value;
  /**
   * The early-exit code, or `VM_EXEC2_NO_EARLY_EXIT`.
   */
  uint64_t early_exit;
} vm_exec2_hook_result_t;

/**
 * A buffer of this size holds every message the executor currently produces.
 *
 * Exported so the Go side has one place to take the number from rather than picking its
 * own.
 */
extern const int VM_EXEC2_ERROR_BUF_RECOMMENDED_LEN;

/**
 * Creates an executor from one configuration struct and one hook table.
 *
 * `hook_table` is copied, so the caller is free to release it as soon as this returns.
 * `hook_table_len` is the number of pointer-sized entries in it.
 *
 * On success writes the executor to `executor_out` and returns `VM_EXEC2_OK`. The caller
 * owns the result and must release it with `vm_exec2_executor_destroy`.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_new_executor(struct vm_exec2_executor_t **executor_out, const struct vm_exec2_executor_config_t *config, const void *hook_table, uintptr_t hook_table_len, char *err_buf, int err_buf_len);

/**
 * Destroys an executor. Does nothing if `executor` is null.
 *
 * Instances created from an executor do not borrow it, so the order of destruction between
 * the two does not matter.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
void vm_exec2_executor_destroy(struct vm_exec2_executor_t *executor);

/**
 * Creates an instance.
 *
 * `go_vm_hooks` is the Go VM hooks object for *this* instance; it is handed back to every
 * VM hook as field 0 of the hook context. It must stay alive for the instance's lifetime.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_new_instance(struct vm_exec2_executor_t *executor, struct vm_exec2_instance_t **instance_out, const uint8_t *wasm_bytes, uint32_t wasm_bytes_len, const struct vm_exec2_compilation_options_t *options, void *go_vm_hooks, char *err_buf, int err_buf_len);

/**
 * Destroys an instance. Does nothing if `instance` is null.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
void vm_exec2_instance_destroy(struct vm_exec2_instance_t *instance);

/**
 * Calls an exported function, with a gas limit for this call.
 *
 * Always writes `result_out` when it returns `VM_EXEC2_OK`. A non-`OK` status means the
 * call could not be attempted at all, which is distinct from a call that was attempted and
 * failed — the latter is reported through `result_out.kind`.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_instance_call(struct vm_exec2_instance_t *instance, const char *func_name, uint64_t points_limit, struct vm_exec2_call_result_t *result_out, char *err_buf, int err_buf_len);

/**
 * Returns the number of points (gas) used so far.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_instance_get_points_used(struct vm_exec2_instance_t *instance, uint64_t *points_out, char *err_buf, int err_buf_len);

/**
 * Checks that every exported function takes no arguments and returns nothing.
 *
 * Returns 1 if so, 0 if not, or a negative status.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
int vm_exec2_instance_check_signatures(const struct vm_exec2_instance_t *instance);

/**
 * Checks whether the contract exports a function with the given name.
 *
 * Returns 1 if so, 0 if not, or a negative status.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
int vm_exec2_instance_has_function(const struct vm_exec2_instance_t *instance, const char *func_name);

/**
 * Buffer length needed by `vm_exec2_instance_exported_function_names`.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
int vm_exec2_instance_exported_function_names_length(const struct vm_exec2_instance_t *instance);

/**
 * Writes the exported function names, pipe-separated, e.g. `"init|endpoint1|endpoint2"`.
 *
 * No ordering is guaranteed. Returns the number of bytes written including the terminator,
 * or `-1` if the buffer is absent or too small.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
int vm_exec2_instance_exported_function_names(const struct vm_exec2_instance_t *instance, char *dest, int dest_len);

/**
 * Buffer length needed by `vm_exec2_instance_imported_function_names`.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
int vm_exec2_instance_imported_function_names_length(const struct vm_exec2_instance_t *instance);

/**
 * Writes the names of the VM hooks the contract imports, pipe-separated.
 *
 * All of them at once, rather than a per-name query, because the Go contract validator
 * checks the module against whole sets of allowed names.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
int vm_exec2_instance_imported_function_names(const struct vm_exec2_instance_t *instance, char *dest, int dest_len);

/**
 * Loads from instance memory into a caller-provided buffer, **truncating** at the end of
 * the memory.
 *
 * Returns the number of bytes actually written, which may be fewer than `dest_len`, or a
 * negative status. The truncation is where the Go VM's historical load semantics live: a
 * load whose offset is valid but whose end lies past the end of the memory yields the bytes
 * that are available and no error. Contracts have observed that, so the executor keeps
 * offering it. An offset outside the memory, or a negative length, is still an error.
 *
 * `dest_len` doubles as the requested length, which is what makes the clamping visible to
 * the caller: Go allocates `length` bytes and gets back `min(length, available)`.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
int64_t vm_exec2_state_mem_load(struct vm_exec2_hook_context_t *ctx, int32_t mem_ptr, uint8_t *dest, int32_t dest_len, char *err_buf, int err_buf_len);

/**
 * Writes to instance memory. The range must fit entirely inside the memory.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_state_mem_store(struct vm_exec2_hook_context_t *ctx, int32_t mem_ptr, const uint8_t *src, int32_t src_len, char *err_buf, int err_buf_len);

/**
 * Returns the size in bytes of the instance memory.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_state_mem_length(struct vm_exec2_hook_context_t *ctx, uint64_t *length_out, char *err_buf, int err_buf_len);

/**
 * Grows the instance memory by `by_num_pages` pages of 64Kb each.
 *
 * Writes the number of pages the memory had *before* growing to `prev_pages_out`.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_state_mem_grow(struct vm_exec2_hook_context_t *ctx, uint32_t by_num_pages, uint32_t *prev_pages_out, char *err_buf, int err_buf_len);

/**
 * Returns the number of points (gas) used by the instance so far.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_state_get_points_used(struct vm_exec2_hook_context_t *ctx, uint64_t *points_out, char *err_buf, int err_buf_len);

/**
 * Sets the number of points (gas) used by the instance.
 *
 * # Safety
 *
 * C API function, works with raw object pointers.
 */
enum vm_exec2_status_t vm_exec2_state_set_points_used(struct vm_exec2_hook_context_t *ctx, uint64_t points, char *err_buf, int err_buf_len);
