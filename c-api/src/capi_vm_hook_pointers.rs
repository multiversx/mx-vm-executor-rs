// Code generated by vmhooks generator. DO NOT EDIT.

// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// !!!!!!!!!!!!!!!!!!!!!! AUTO-GENERATED FILE !!!!!!!!!!!!!!!!!!!!!!
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

use std::ffi::c_void;

#[repr(C)]
#[derive(Clone)]
#[rustfmt::skip]
pub struct vm_exec_vm_hook_c_func_pointers {
    pub get_gas_left_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_sc_address_func_ptr: extern "C" fn(context: *mut c_void, result_offset: i32),
    pub get_owner_address_func_ptr: extern "C" fn(context: *mut c_void, result_offset: i32),
    pub get_shard_of_address_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32) -> i32,
    pub is_smart_contract_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32) -> i32,
    pub signal_error_func_ptr: extern "C" fn(context: *mut c_void, message_offset: i32, message_length: i32),
    pub get_external_balance_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, result_offset: i32),
    pub get_block_hash_func_ptr: extern "C" fn(context: *mut c_void, nonce: i64, result_offset: i32) -> i32,
    pub get_esdt_balance_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64, result_offset: i32) -> i32,
    pub get_esdt_nft_name_length_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64) -> i32,
    pub get_esdt_nft_attribute_length_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64) -> i32,
    pub get_esdt_nft_uri_length_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64) -> i32,
    pub get_esdt_token_data_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64, value_handle: i32, properties_offset: i32, hash_offset: i32, name_offset: i32, attributes_offset: i32, creator_offset: i32, royalties_handle: i32, uris_offset: i32) -> i32,
    pub get_esdt_local_roles_func_ptr: extern "C" fn(context: *mut c_void, token_id_handle: i32) -> i64,
    pub validate_token_identifier_func_ptr: extern "C" fn(context: *mut c_void, token_id_handle: i32) -> i32,
    pub transfer_value_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, value_offset: i32, data_offset: i32, length: i32) -> i32,
    pub transfer_value_execute_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, value_offset: i32, gas_limit: i64, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub transfer_esdt_execute_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, token_id_offset: i32, token_id_len: i32, value_offset: i32, gas_limit: i64, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub transfer_esdt_nft_execute_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, token_id_offset: i32, token_id_len: i32, value_offset: i32, nonce: i64, gas_limit: i64, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub multi_transfer_esdt_nft_execute_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, num_token_transfers: i32, token_transfers_args_length_offset: i32, token_transfer_data_offset: i32, gas_limit: i64, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub create_async_call_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, value_offset: i32, data_offset: i32, data_length: i32, success_offset: i32, success_length: i32, error_offset: i32, error_length: i32, gas: i64, extra_gas_for_callback: i64) -> i32,
    pub set_async_context_callback_func_ptr: extern "C" fn(context: *mut c_void, callback: i32, callback_length: i32, data: i32, data_length: i32, gas: i64) -> i32,
    pub upgrade_contract_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, gas_limit: i64, value_offset: i32, code_offset: i32, code_metadata_offset: i32, length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32),
    pub upgrade_from_source_contract_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, gas_limit: i64, value_offset: i32, source_contract_address_offset: i32, code_metadata_offset: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32),
    pub delete_contract_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, gas_limit: i64, num_arguments: i32, arguments_length_offset: i32, data_offset: i32),
    pub async_call_func_ptr: extern "C" fn(context: *mut c_void, dest_offset: i32, value_offset: i32, data_offset: i32, length: i32),
    pub get_argument_length_func_ptr: extern "C" fn(context: *mut c_void, id: i32) -> i32,
    pub get_argument_func_ptr: extern "C" fn(context: *mut c_void, id: i32, arg_offset: i32) -> i32,
    pub get_function_func_ptr: extern "C" fn(context: *mut c_void, function_offset: i32) -> i32,
    pub get_num_arguments_func_ptr: extern "C" fn(context: *mut c_void) -> i32,
    pub storage_store_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, data_offset: i32, data_length: i32) -> i32,
    pub storage_load_length_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32) -> i32,
    pub storage_load_from_address_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, key_offset: i32, key_length: i32, data_offset: i32) -> i32,
    pub storage_load_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, data_offset: i32) -> i32,
    pub set_storage_lock_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, lock_timestamp: i64) -> i32,
    pub get_storage_lock_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32) -> i64,
    pub is_storage_locked_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32) -> i32,
    pub clear_storage_lock_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32) -> i32,
    pub get_caller_func_ptr: extern "C" fn(context: *mut c_void, result_offset: i32),
    pub check_no_payment_func_ptr: extern "C" fn(context: *mut c_void),
    pub get_call_value_func_ptr: extern "C" fn(context: *mut c_void, result_offset: i32) -> i32,
    pub get_esdt_value_func_ptr: extern "C" fn(context: *mut c_void, result_offset: i32) -> i32,
    pub get_esdt_value_by_index_func_ptr: extern "C" fn(context: *mut c_void, result_offset: i32, index: i32) -> i32,
    pub get_esdt_token_name_func_ptr: extern "C" fn(context: *mut c_void, result_offset: i32) -> i32,
    pub get_esdt_token_name_by_index_func_ptr: extern "C" fn(context: *mut c_void, result_offset: i32, index: i32) -> i32,
    pub get_esdt_token_nonce_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_esdt_token_nonce_by_index_func_ptr: extern "C" fn(context: *mut c_void, index: i32) -> i64,
    pub get_current_esdt_nft_nonce_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, token_id_offset: i32, token_id_len: i32) -> i64,
    pub get_esdt_token_type_func_ptr: extern "C" fn(context: *mut c_void) -> i32,
    pub get_esdt_token_type_by_index_func_ptr: extern "C" fn(context: *mut c_void, index: i32) -> i32,
    pub get_num_esdt_transfers_func_ptr: extern "C" fn(context: *mut c_void) -> i32,
    pub get_call_value_token_name_func_ptr: extern "C" fn(context: *mut c_void, call_value_offset: i32, token_name_offset: i32) -> i32,
    pub get_call_value_token_name_by_index_func_ptr: extern "C" fn(context: *mut c_void, call_value_offset: i32, token_name_offset: i32, index: i32) -> i32,
    pub write_log_func_ptr: extern "C" fn(context: *mut c_void, data_pointer: i32, data_length: i32, topic_ptr: i32, num_topics: i32),
    pub write_event_log_func_ptr: extern "C" fn(context: *mut c_void, num_topics: i32, topic_lengths_offset: i32, topic_offset: i32, data_offset: i32, data_length: i32),
    pub get_block_timestamp_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_block_nonce_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_block_round_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_block_epoch_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_block_random_seed_func_ptr: extern "C" fn(context: *mut c_void, pointer: i32),
    pub get_state_root_hash_func_ptr: extern "C" fn(context: *mut c_void, pointer: i32),
    pub get_prev_block_timestamp_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_prev_block_nonce_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_prev_block_round_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_prev_block_epoch_func_ptr: extern "C" fn(context: *mut c_void) -> i64,
    pub get_prev_block_random_seed_func_ptr: extern "C" fn(context: *mut c_void, pointer: i32),
    pub finish_func_ptr: extern "C" fn(context: *mut c_void, pointer: i32, length: i32),
    pub execute_on_same_context_func_ptr: extern "C" fn(context: *mut c_void, gas_limit: i64, address_offset: i32, value_offset: i32, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub execute_on_dest_context_func_ptr: extern "C" fn(context: *mut c_void, gas_limit: i64, address_offset: i32, value_offset: i32, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub execute_read_only_func_ptr: extern "C" fn(context: *mut c_void, gas_limit: i64, address_offset: i32, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub create_contract_func_ptr: extern "C" fn(context: *mut c_void, gas_limit: i64, value_offset: i32, code_offset: i32, code_metadata_offset: i32, length: i32, result_offset: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub deploy_from_source_contract_func_ptr: extern "C" fn(context: *mut c_void, gas_limit: i64, value_offset: i32, source_contract_address_offset: i32, code_metadata_offset: i32, result_address_offset: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32,
    pub get_num_return_data_func_ptr: extern "C" fn(context: *mut c_void) -> i32,
    pub get_return_data_size_func_ptr: extern "C" fn(context: *mut c_void, result_id: i32) -> i32,
    pub get_return_data_func_ptr: extern "C" fn(context: *mut c_void, result_id: i32, data_offset: i32) -> i32,
    pub clean_return_data_func_ptr: extern "C" fn(context: *mut c_void),
    pub delete_from_return_data_func_ptr: extern "C" fn(context: *mut c_void, result_id: i32),
    pub get_original_tx_hash_func_ptr: extern "C" fn(context: *mut c_void, data_offset: i32),
    pub get_current_tx_hash_func_ptr: extern "C" fn(context: *mut c_void, data_offset: i32),
    pub get_prev_tx_hash_func_ptr: extern "C" fn(context: *mut c_void, data_offset: i32),
    pub managed_sc_address_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32),
    pub managed_owner_address_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32),
    pub managed_caller_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32),
    pub managed_signal_error_func_ptr: extern "C" fn(context: *mut c_void, err_handle: i32),
    pub managed_write_log_func_ptr: extern "C" fn(context: *mut c_void, topics_handle: i32, data_handle: i32),
    pub managed_get_original_tx_hash_func_ptr: extern "C" fn(context: *mut c_void, result_handle: i32),
    pub managed_get_state_root_hash_func_ptr: extern "C" fn(context: *mut c_void, result_handle: i32),
    pub managed_get_block_random_seed_func_ptr: extern "C" fn(context: *mut c_void, result_handle: i32),
    pub managed_get_prev_block_random_seed_func_ptr: extern "C" fn(context: *mut c_void, result_handle: i32),
    pub managed_get_return_data_func_ptr: extern "C" fn(context: *mut c_void, result_id: i32, result_handle: i32),
    pub managed_get_multi_esdt_call_value_func_ptr: extern "C" fn(context: *mut c_void, multi_call_value_handle: i32),
    pub managed_get_esdt_balance_func_ptr: extern "C" fn(context: *mut c_void, address_handle: i32, token_id_handle: i32, nonce: i64, value_handle: i32),
    pub managed_get_esdt_token_data_func_ptr: extern "C" fn(context: *mut c_void, address_handle: i32, token_id_handle: i32, nonce: i64, value_handle: i32, properties_handle: i32, hash_handle: i32, name_handle: i32, attributes_handle: i32, creator_handle: i32, royalties_handle: i32, uris_handle: i32),
    pub managed_async_call_func_ptr: extern "C" fn(context: *mut c_void, dest_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32),
    pub managed_create_async_call_func_ptr: extern "C" fn(context: *mut c_void, dest_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32, success_offset: i32, success_length: i32, error_offset: i32, error_length: i32, gas: i64, extra_gas_for_callback: i64, callback_closure_handle: i32) -> i32,
    pub managed_get_callback_closure_func_ptr: extern "C" fn(context: *mut c_void, callback_closure_handle: i32),
    pub managed_upgrade_from_source_contract_func_ptr: extern "C" fn(context: *mut c_void, dest_handle: i32, gas: i64, value_handle: i32, address_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_handle: i32),
    pub managed_upgrade_contract_func_ptr: extern "C" fn(context: *mut c_void, dest_handle: i32, gas: i64, value_handle: i32, code_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_handle: i32),
    pub managed_delete_contract_func_ptr: extern "C" fn(context: *mut c_void, dest_handle: i32, gas_limit: i64, arguments_handle: i32),
    pub managed_deploy_from_source_contract_func_ptr: extern "C" fn(context: *mut c_void, gas: i64, value_handle: i32, address_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_address_handle: i32, result_handle: i32) -> i32,
    pub managed_create_contract_func_ptr: extern "C" fn(context: *mut c_void, gas: i64, value_handle: i32, code_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_address_handle: i32, result_handle: i32) -> i32,
    pub managed_execute_read_only_func_ptr: extern "C" fn(context: *mut c_void, gas: i64, address_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32,
    pub managed_execute_on_same_context_func_ptr: extern "C" fn(context: *mut c_void, gas: i64, address_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32,
    pub managed_execute_on_dest_context_func_ptr: extern "C" fn(context: *mut c_void, gas: i64, address_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32,
    pub managed_multi_transfer_esdt_nft_execute_func_ptr: extern "C" fn(context: *mut c_void, dst_handle: i32, token_transfers_handle: i32, gas_limit: i64, function_handle: i32, arguments_handle: i32) -> i32,
    pub managed_transfer_value_execute_func_ptr: extern "C" fn(context: *mut c_void, dst_handle: i32, value_handle: i32, gas_limit: i64, function_handle: i32, arguments_handle: i32) -> i32,
    pub managed_is_esdt_frozen_func_ptr: extern "C" fn(context: *mut c_void, address_handle: i32, token_id_handle: i32, nonce: i64) -> i32,
    pub managed_is_esdt_limited_transfer_func_ptr: extern "C" fn(context: *mut c_void, token_id_handle: i32) -> i32,
    pub managed_is_esdt_paused_func_ptr: extern "C" fn(context: *mut c_void, token_id_handle: i32) -> i32,
    pub managed_buffer_to_hex_func_ptr: extern "C" fn(context: *mut c_void, source_handle: i32, dest_handle: i32),
    pub managed_get_code_metadata_func_ptr: extern "C" fn(context: *mut c_void, address_handle: i32, response_handle: i32),
    pub managed_is_builtin_function_func_ptr: extern "C" fn(context: *mut c_void, function_name_handle: i32) -> i32,
    pub big_float_new_from_parts_func_ptr: extern "C" fn(context: *mut c_void, integral_part: i32, fractional_part: i32, exponent: i32) -> i32,
    pub big_float_new_from_frac_func_ptr: extern "C" fn(context: *mut c_void, numerator: i64, denominator: i64) -> i32,
    pub big_float_new_from_sci_func_ptr: extern "C" fn(context: *mut c_void, significand: i64, exponent: i64) -> i32,
    pub big_float_add_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_float_sub_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_float_mul_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_float_div_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_float_neg_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32),
    pub big_float_clone_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32),
    pub big_float_cmp_func_ptr: extern "C" fn(context: *mut c_void, op1_handle: i32, op2_handle: i32) -> i32,
    pub big_float_abs_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32),
    pub big_float_sign_func_ptr: extern "C" fn(context: *mut c_void, op_handle: i32) -> i32,
    pub big_float_sqrt_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32),
    pub big_float_pow_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32, exponent: i32),
    pub big_float_floor_func_ptr: extern "C" fn(context: *mut c_void, dest_big_int_handle: i32, op_handle: i32),
    pub big_float_ceil_func_ptr: extern "C" fn(context: *mut c_void, dest_big_int_handle: i32, op_handle: i32),
    pub big_float_truncate_func_ptr: extern "C" fn(context: *mut c_void, dest_big_int_handle: i32, op_handle: i32),
    pub big_float_set_int64_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, value: i64),
    pub big_float_is_int_func_ptr: extern "C" fn(context: *mut c_void, op_handle: i32) -> i32,
    pub big_float_set_big_int_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, big_int_handle: i32),
    pub big_float_get_const_pi_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32),
    pub big_float_get_const_e_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32),
    pub big_int_get_unsigned_argument_func_ptr: extern "C" fn(context: *mut c_void, id: i32, destination_handle: i32),
    pub big_int_get_signed_argument_func_ptr: extern "C" fn(context: *mut c_void, id: i32, destination_handle: i32),
    pub big_int_storage_store_unsigned_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, source_handle: i32) -> i32,
    pub big_int_storage_load_unsigned_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, destination_handle: i32) -> i32,
    pub big_int_get_call_value_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32),
    pub big_int_get_esdt_call_value_func_ptr: extern "C" fn(context: *mut c_void, destination: i32),
    pub big_int_get_esdt_call_value_by_index_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, index: i32),
    pub big_int_get_external_balance_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, result: i32),
    pub big_int_get_esdt_external_balance_func_ptr: extern "C" fn(context: *mut c_void, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64, result_handle: i32),
    pub big_int_new_func_ptr: extern "C" fn(context: *mut c_void, small_value: i64) -> i32,
    pub big_int_unsigned_byte_length_func_ptr: extern "C" fn(context: *mut c_void, reference_handle: i32) -> i32,
    pub big_int_signed_byte_length_func_ptr: extern "C" fn(context: *mut c_void, reference_handle: i32) -> i32,
    pub big_int_get_unsigned_bytes_func_ptr: extern "C" fn(context: *mut c_void, reference_handle: i32, byte_offset: i32) -> i32,
    pub big_int_get_signed_bytes_func_ptr: extern "C" fn(context: *mut c_void, reference_handle: i32, byte_offset: i32) -> i32,
    pub big_int_set_unsigned_bytes_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, byte_offset: i32, byte_length: i32),
    pub big_int_set_signed_bytes_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, byte_offset: i32, byte_length: i32),
    pub big_int_is_int64_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32) -> i32,
    pub big_int_get_int64_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32) -> i64,
    pub big_int_set_int64_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, value: i64),
    pub big_int_add_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_sub_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_mul_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_tdiv_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_tmod_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_ediv_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_emod_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_sqrt_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32),
    pub big_int_pow_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_log2_func_ptr: extern "C" fn(context: *mut c_void, op1_handle: i32) -> i32,
    pub big_int_abs_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32),
    pub big_int_neg_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32),
    pub big_int_sign_func_ptr: extern "C" fn(context: *mut c_void, op_handle: i32) -> i32,
    pub big_int_cmp_func_ptr: extern "C" fn(context: *mut c_void, op1_handle: i32, op2_handle: i32) -> i32,
    pub big_int_not_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32),
    pub big_int_and_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_or_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_xor_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op1_handle: i32, op2_handle: i32),
    pub big_int_shr_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32, bits: i32),
    pub big_int_shl_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, op_handle: i32, bits: i32),
    pub big_int_finish_unsigned_func_ptr: extern "C" fn(context: *mut c_void, reference_handle: i32),
    pub big_int_finish_signed_func_ptr: extern "C" fn(context: *mut c_void, reference_handle: i32),
    pub big_int_to_string_func_ptr: extern "C" fn(context: *mut c_void, big_int_handle: i32, destination_handle: i32),
    pub mbuffer_new_func_ptr: extern "C" fn(context: *mut c_void) -> i32,
    pub mbuffer_new_from_bytes_func_ptr: extern "C" fn(context: *mut c_void, data_offset: i32, data_length: i32) -> i32,
    pub mbuffer_get_length_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32) -> i32,
    pub mbuffer_get_bytes_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, result_offset: i32) -> i32,
    pub mbuffer_get_byte_slice_func_ptr: extern "C" fn(context: *mut c_void, source_handle: i32, starting_position: i32, slice_length: i32, result_offset: i32) -> i32,
    pub mbuffer_copy_byte_slice_func_ptr: extern "C" fn(context: *mut c_void, source_handle: i32, starting_position: i32, slice_length: i32, destination_handle: i32) -> i32,
    pub mbuffer_eq_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle1: i32, m_buffer_handle2: i32) -> i32,
    pub mbuffer_set_bytes_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, data_offset: i32, data_length: i32) -> i32,
    pub mbuffer_set_byte_slice_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, starting_position: i32, data_length: i32, data_offset: i32) -> i32,
    pub mbuffer_append_func_ptr: extern "C" fn(context: *mut c_void, accumulator_handle: i32, data_handle: i32) -> i32,
    pub mbuffer_append_bytes_func_ptr: extern "C" fn(context: *mut c_void, accumulator_handle: i32, data_offset: i32, data_length: i32) -> i32,
    pub mbuffer_to_big_int_unsigned_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, big_int_handle: i32) -> i32,
    pub mbuffer_to_big_int_signed_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, big_int_handle: i32) -> i32,
    pub mbuffer_from_big_int_unsigned_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, big_int_handle: i32) -> i32,
    pub mbuffer_from_big_int_signed_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, big_int_handle: i32) -> i32,
    pub mbuffer_to_big_float_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, big_float_handle: i32) -> i32,
    pub mbuffer_from_big_float_func_ptr: extern "C" fn(context: *mut c_void, m_buffer_handle: i32, big_float_handle: i32) -> i32,
    pub mbuffer_storage_store_func_ptr: extern "C" fn(context: *mut c_void, key_handle: i32, source_handle: i32) -> i32,
    pub mbuffer_storage_load_func_ptr: extern "C" fn(context: *mut c_void, key_handle: i32, destination_handle: i32) -> i32,
    pub mbuffer_storage_load_from_address_func_ptr: extern "C" fn(context: *mut c_void, address_handle: i32, key_handle: i32, destination_handle: i32),
    pub mbuffer_get_argument_func_ptr: extern "C" fn(context: *mut c_void, id: i32, destination_handle: i32) -> i32,
    pub mbuffer_finish_func_ptr: extern "C" fn(context: *mut c_void, source_handle: i32) -> i32,
    pub mbuffer_set_random_func_ptr: extern "C" fn(context: *mut c_void, destination_handle: i32, length: i32) -> i32,
    pub managed_map_new_func_ptr: extern "C" fn(context: *mut c_void) -> i32,
    pub managed_map_put_func_ptr: extern "C" fn(context: *mut c_void, m_map_handle: i32, key_handle: i32, value_handle: i32) -> i32,
    pub managed_map_get_func_ptr: extern "C" fn(context: *mut c_void, m_map_handle: i32, key_handle: i32, out_value_handle: i32) -> i32,
    pub managed_map_remove_func_ptr: extern "C" fn(context: *mut c_void, m_map_handle: i32, key_handle: i32, out_value_handle: i32) -> i32,
    pub managed_map_contains_func_ptr: extern "C" fn(context: *mut c_void, m_map_handle: i32, key_handle: i32) -> i32,
    pub small_int_get_unsigned_argument_func_ptr: extern "C" fn(context: *mut c_void, id: i32) -> i64,
    pub small_int_get_signed_argument_func_ptr: extern "C" fn(context: *mut c_void, id: i32) -> i64,
    pub small_int_finish_unsigned_func_ptr: extern "C" fn(context: *mut c_void, value: i64),
    pub small_int_finish_signed_func_ptr: extern "C" fn(context: *mut c_void, value: i64),
    pub small_int_storage_store_unsigned_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, value: i64) -> i32,
    pub small_int_storage_store_signed_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, value: i64) -> i32,
    pub small_int_storage_load_unsigned_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32) -> i64,
    pub small_int_storage_load_signed_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32) -> i64,
    pub int64get_argument_func_ptr: extern "C" fn(context: *mut c_void, id: i32) -> i64,
    pub int64finish_func_ptr: extern "C" fn(context: *mut c_void, value: i64),
    pub int64storage_store_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, value: i64) -> i32,
    pub int64storage_load_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32) -> i64,
    pub sha256_func_ptr: extern "C" fn(context: *mut c_void, data_offset: i32, length: i32, result_offset: i32) -> i32,
    pub managed_sha256_func_ptr: extern "C" fn(context: *mut c_void, input_handle: i32, output_handle: i32) -> i32,
    pub keccak256_func_ptr: extern "C" fn(context: *mut c_void, data_offset: i32, length: i32, result_offset: i32) -> i32,
    pub managed_keccak256_func_ptr: extern "C" fn(context: *mut c_void, input_handle: i32, output_handle: i32) -> i32,
    pub ripemd160_func_ptr: extern "C" fn(context: *mut c_void, data_offset: i32, length: i32, result_offset: i32) -> i32,
    pub managed_ripemd160_func_ptr: extern "C" fn(context: *mut c_void, input_handle: i32, output_handle: i32) -> i32,
    pub verify_bls_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, message_offset: i32, message_length: i32, sig_offset: i32) -> i32,
    pub managed_verify_bls_func_ptr: extern "C" fn(context: *mut c_void, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32,
    pub verify_ed25519_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, message_offset: i32, message_length: i32, sig_offset: i32) -> i32,
    pub managed_verify_ed25519_func_ptr: extern "C" fn(context: *mut c_void, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32,
    pub verify_custom_secp256k1_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, message_offset: i32, message_length: i32, sig_offset: i32, hash_type: i32) -> i32,
    pub managed_verify_custom_secp256k1_func_ptr: extern "C" fn(context: *mut c_void, key_handle: i32, message_handle: i32, sig_handle: i32, hash_type: i32) -> i32,
    pub verify_secp256k1_func_ptr: extern "C" fn(context: *mut c_void, key_offset: i32, key_length: i32, message_offset: i32, message_length: i32, sig_offset: i32) -> i32,
    pub managed_verify_secp256k1_func_ptr: extern "C" fn(context: *mut c_void, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32,
    pub encode_secp256k1_der_signature_func_ptr: extern "C" fn(context: *mut c_void, r_offset: i32, r_length: i32, s_offset: i32, s_length: i32, sig_offset: i32) -> i32,
    pub managed_encode_secp256k1_der_signature_func_ptr: extern "C" fn(context: *mut c_void, r_handle: i32, s_handle: i32, sig_handle: i32) -> i32,
    pub add_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, fst_point_xhandle: i32, fst_point_yhandle: i32, snd_point_xhandle: i32, snd_point_yhandle: i32),
    pub double_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32),
    pub is_on_curve_ec_func_ptr: extern "C" fn(context: *mut c_void, ec_handle: i32, point_xhandle: i32, point_yhandle: i32) -> i32,
    pub scalar_base_mult_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: i32, length: i32) -> i32,
    pub managed_scalar_base_mult_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32,
    pub scalar_mult_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32, data_offset: i32, length: i32) -> i32,
    pub managed_scalar_mult_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32, data_handle: i32) -> i32,
    pub marshal_ec_func_ptr: extern "C" fn(context: *mut c_void, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_offset: i32) -> i32,
    pub managed_marshal_ec_func_ptr: extern "C" fn(context: *mut c_void, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_handle: i32) -> i32,
    pub marshal_compressed_ec_func_ptr: extern "C" fn(context: *mut c_void, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_offset: i32) -> i32,
    pub managed_marshal_compressed_ec_func_ptr: extern "C" fn(context: *mut c_void, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_handle: i32) -> i32,
    pub unmarshal_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: i32, length: i32) -> i32,
    pub managed_unmarshal_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32,
    pub unmarshal_compressed_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: i32, length: i32) -> i32,
    pub managed_unmarshal_compressed_ec_func_ptr: extern "C" fn(context: *mut c_void, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32,
    pub generate_key_ec_func_ptr: extern "C" fn(context: *mut c_void, x_pub_key_handle: i32, y_pub_key_handle: i32, ec_handle: i32, result_offset: i32) -> i32,
    pub managed_generate_key_ec_func_ptr: extern "C" fn(context: *mut c_void, x_pub_key_handle: i32, y_pub_key_handle: i32, ec_handle: i32, result_handle: i32) -> i32,
    pub create_ec_func_ptr: extern "C" fn(context: *mut c_void, data_offset: i32, data_length: i32) -> i32,
    pub managed_create_ec_func_ptr: extern "C" fn(context: *mut c_void, data_handle: i32) -> i32,
    pub get_curve_length_ec_func_ptr: extern "C" fn(context: *mut c_void, ec_handle: i32) -> i32,
    pub get_priv_key_byte_length_ec_func_ptr: extern "C" fn(context: *mut c_void, ec_handle: i32) -> i32,
    pub elliptic_curve_get_values_func_ptr: extern "C" fn(context: *mut c_void, ec_handle: i32, field_order_handle: i32, base_point_order_handle: i32, eq_constant_handle: i32, x_base_point_handle: i32, y_base_point_handle: i32) -> i32,
}

impl std::fmt::Debug for vm_exec_vm_hook_c_func_pointers {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "vm_exec_vm_hook_c_func_pointers")
    }
}