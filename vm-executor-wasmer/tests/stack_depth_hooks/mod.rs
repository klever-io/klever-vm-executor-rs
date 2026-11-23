//! VM hooks for the nested call stack depth tests: `getGasLeft` calls a function on
//! another instance on the same thread, the way the Go node runs a contract-to-contract
//! call while the calling contract is still executing. Every other hook delegates to
//! `VMHooksDefault`; the delegating methods are generated from the `VMHooks` trait.

use klever_chain_vm_executor::{Instance, MemLength, MemPtr, VMHooks, VMHooksDefault};
use klever_chain_vm_executor_wasmer::{last_call_stack_depth_stats, StackDepthStats};
use std::cell::RefCell;
use std::ffi::c_void;
use std::fmt;
use std::rc::Rc;

/// The outcome of one nested call made from `getGasLeft`.
pub type NestedCallRecord = (Result<(), String>, Option<StackDepthStats>);

/// State shared between a test and the hooks of the calling instance.
#[derive(Default)]
pub struct NestedCallTarget {
    /// The instance `getGasLeft` calls into. `None` makes `getGasLeft` a no-op.
    pub instance: Option<Rc<dyn Instance>>,
    pub func_name: String,
    pub records: Vec<NestedCallRecord>,
}

#[derive(Clone)]
pub struct NestedCallHooks {
    pub target: Rc<RefCell<NestedCallTarget>>,
}

impl fmt::Debug for NestedCallHooks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NestedCallHooks").finish()
    }
}

#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
impl VMHooks for NestedCallHooks {
    fn set_vm_hooks_ptr(&mut self, _vm_hooks_ptr: *mut c_void) {}

    fn get_gas_left(&self) -> i64 {
        // No RefCell borrow is held across the nested call: the nested instance may call
        // back into these hooks.
        let (instance, func_name) = {
            let target = self.target.borrow();
            (target.instance.clone(), target.func_name.clone())
        };
        if let Some(instance) = instance {
            let result = instance.call(&func_name);
            let stats = last_call_stack_depth_stats();
            self.target.borrow_mut().records.push((result, stats));
        }
        0
    }

    fn get_sc_address(&self, result_offset: MemPtr) {
        VMHooksDefault.get_sc_address(result_offset)
    }

    fn get_owner_address(&self, result_offset: MemPtr) {
        VMHooksDefault.get_owner_address(result_offset)
    }

    fn is_smart_contract(&self, address_offset: MemPtr) -> i32 {
        VMHooksDefault.is_smart_contract(address_offset)
    }

    fn signal_error(&self, message_offset: MemPtr, message_length: MemLength) {
        VMHooksDefault.signal_error(message_offset, message_length)
    }

    fn get_external_balance(&self, address_offset: MemPtr, result_offset: MemPtr) {
        VMHooksDefault.get_external_balance(address_offset, result_offset)
    }

    fn get_block_hash(&self, nonce: i64, result_offset: MemPtr) -> i32 {
        VMHooksDefault.get_block_hash(nonce, result_offset)
    }

    fn get_kda_balance(&self, address_offset: MemPtr, token_id_offset: MemPtr, token_id_len: MemLength, nonce: i64, result_offset: MemPtr) -> i32 {
        VMHooksDefault.get_kda_balance(address_offset, token_id_offset, token_id_len, nonce, result_offset)
    }

    fn get_kda_name_length(&self, address_offset: MemPtr, token_id_offset: MemPtr, token_id_len: MemLength, nonce: i64) -> i32 {
        VMHooksDefault.get_kda_name_length(address_offset, token_id_offset, token_id_len, nonce)
    }

    fn get_kda_uri_length(&self, address_offset: MemPtr, token_id_offset: MemPtr, token_id_len: MemLength, nonce: i64) -> i32 {
        VMHooksDefault.get_kda_uri_length(address_offset, token_id_offset, token_id_len, nonce)
    }

    fn get_kda_token_data(&self, address_offset: MemPtr, token_id_offset: MemPtr, token_id_len: MemLength, nonce: i64, precision_handle: i32, id_offset: MemPtr, name_offset: MemPtr, creator_offset: MemPtr, logo_offset: MemPtr, initial_supply_offset: MemPtr, circulating_supply_offset: MemPtr, max_supply_offset: MemPtr, minted_offset: MemPtr, burned_offset: MemPtr, royalties_offset: MemPtr, properties_offset: MemPtr, attributes_offset: MemPtr, roles_offset: MemPtr) -> i32 {
        VMHooksDefault.get_kda_token_data(address_offset, token_id_offset, token_id_len, nonce, precision_handle, id_offset, name_offset, creator_offset, logo_offset, initial_supply_offset, circulating_supply_offset, max_supply_offset, minted_offset, burned_offset, royalties_offset, properties_offset, attributes_offset, roles_offset)
    }

    fn validate_token_identifier(&self, token_id_handle: i32) -> i32 {
        VMHooksDefault.validate_token_identifier(token_id_handle)
    }

    fn upgrade_contract(&self, dest_offset: MemPtr, gas_limit: i64, value_offset: MemPtr, code_offset: MemPtr, code_metadata_offset: MemPtr, length: MemLength, num_arguments: i32, arguments_length_offset: MemPtr, data_offset: MemPtr) {
        VMHooksDefault.upgrade_contract(dest_offset, gas_limit, value_offset, code_offset, code_metadata_offset, length, num_arguments, arguments_length_offset, data_offset)
    }

    fn upgrade_from_source_contract(&self, dest_offset: MemPtr, gas_limit: i64, value_offset: MemPtr, source_contract_address_offset: MemPtr, code_metadata_offset: MemPtr, num_arguments: i32, arguments_length_offset: MemPtr, data_offset: MemPtr) {
        VMHooksDefault.upgrade_from_source_contract(dest_offset, gas_limit, value_offset, source_contract_address_offset, code_metadata_offset, num_arguments, arguments_length_offset, data_offset)
    }

    fn delete_contract(&self, dest_offset: MemPtr, gas_limit: i64, num_arguments: i32, arguments_length_offset: MemPtr, data_offset: MemPtr) {
        VMHooksDefault.delete_contract(dest_offset, gas_limit, num_arguments, arguments_length_offset, data_offset)
    }

    fn get_argument_length(&self, id: i32) -> i32 {
        VMHooksDefault.get_argument_length(id)
    }

    fn get_argument(&self, id: i32, arg_offset: MemPtr) -> i32 {
        VMHooksDefault.get_argument(id, arg_offset)
    }

    fn get_function(&self, function_offset: MemPtr) -> i32 {
        VMHooksDefault.get_function(function_offset)
    }

    fn get_num_arguments(&self) -> i32 {
        VMHooksDefault.get_num_arguments()
    }

    fn storage_store(&self, key_offset: MemPtr, key_length: MemLength, data_offset: MemPtr, data_length: MemLength) -> i32 {
        VMHooksDefault.storage_store(key_offset, key_length, data_offset, data_length)
    }

    fn storage_load_length(&self, key_offset: MemPtr, key_length: MemLength) -> i32 {
        VMHooksDefault.storage_load_length(key_offset, key_length)
    }

    fn storage_load_from_address(&self, address_offset: MemPtr, key_offset: MemPtr, key_length: MemLength, data_offset: MemPtr) -> i32 {
        VMHooksDefault.storage_load_from_address(address_offset, key_offset, key_length, data_offset)
    }

    fn storage_load(&self, key_offset: MemPtr, key_length: MemLength, data_offset: MemPtr) -> i32 {
        VMHooksDefault.storage_load(key_offset, key_length, data_offset)
    }

    fn set_storage_lock(&self, key_offset: MemPtr, key_length: MemLength, lock_timestamp: i64) -> i32 {
        VMHooksDefault.set_storage_lock(key_offset, key_length, lock_timestamp)
    }

    fn get_storage_lock(&self, key_offset: MemPtr, key_length: MemLength) -> i64 {
        VMHooksDefault.get_storage_lock(key_offset, key_length)
    }

    fn is_storage_locked(&self, key_offset: MemPtr, key_length: MemLength) -> i32 {
        VMHooksDefault.is_storage_locked(key_offset, key_length)
    }

    fn clear_storage_lock(&self, key_offset: MemPtr, key_length: MemLength) -> i32 {
        VMHooksDefault.clear_storage_lock(key_offset, key_length)
    }

    fn get_caller(&self, result_offset: MemPtr) {
        VMHooksDefault.get_caller(result_offset)
    }

    fn check_no_payment(&self) {
        VMHooksDefault.check_no_payment()
    }

    fn get_call_value(&self, result_offset: MemPtr) -> i32 {
        VMHooksDefault.get_call_value(result_offset)
    }

    fn get_kda_value(&self, result_offset: MemPtr) -> i32 {
        VMHooksDefault.get_kda_value(result_offset)
    }

    fn get_kda_value_by_index(&self, result_offset: MemPtr, index: i32) -> i32 {
        VMHooksDefault.get_kda_value_by_index(result_offset, index)
    }

    fn get_kda_token_name(&self, result_offset: MemPtr) -> i32 {
        VMHooksDefault.get_kda_token_name(result_offset)
    }

    fn get_kda_token_name_by_index(&self, result_offset: MemPtr, index: i32) -> i32 {
        VMHooksDefault.get_kda_token_name_by_index(result_offset, index)
    }

    fn get_kda_token_nonce(&self) -> i64 {
        VMHooksDefault.get_kda_token_nonce()
    }

    fn get_kda_token_nonce_by_index(&self, index: i32) -> i64 {
        VMHooksDefault.get_kda_token_nonce_by_index(index)
    }

    fn get_kda_token_type(&self) -> i32 {
        VMHooksDefault.get_kda_token_type()
    }

    fn get_kda_token_type_by_index(&self, index: i32) -> i32 {
        VMHooksDefault.get_kda_token_type_by_index(index)
    }

    fn get_num_kda_transfers(&self) -> i32 {
        VMHooksDefault.get_num_kda_transfers()
    }

    fn get_call_value_by_token_name(&self, call_value_offset: MemPtr, token_name_offset: MemPtr, token_name_length: MemLength) -> i32 {
        VMHooksDefault.get_call_value_by_token_name(call_value_offset, token_name_offset, token_name_length)
    }

    fn get_call_value_token_name(&self, call_value_offset: MemPtr, token_name_offset: MemPtr) -> i32 {
        VMHooksDefault.get_call_value_token_name(call_value_offset, token_name_offset)
    }

    fn get_call_value_token_name_by_index(&self, call_value_offset: MemPtr, token_name_offset: MemPtr, index: i32) -> i32 {
        VMHooksDefault.get_call_value_token_name_by_index(call_value_offset, token_name_offset, index)
    }

    fn write_log(&self, data_pointer: MemPtr, data_length: MemLength, topic_ptr: MemPtr, num_topics: i32) {
        VMHooksDefault.write_log(data_pointer, data_length, topic_ptr, num_topics)
    }

    fn write_event_log(&self, num_topics: i32, topic_lengths_offset: MemPtr, topic_offset: MemPtr, data_offset: MemPtr, data_length: MemLength) {
        VMHooksDefault.write_event_log(num_topics, topic_lengths_offset, topic_offset, data_offset, data_length)
    }

    fn get_block_timestamp(&self) -> i64 {
        VMHooksDefault.get_block_timestamp()
    }

    fn get_block_nonce(&self) -> i64 {
        VMHooksDefault.get_block_nonce()
    }

    fn get_block_round(&self) -> i64 {
        VMHooksDefault.get_block_round()
    }

    fn get_block_epoch(&self) -> i64 {
        VMHooksDefault.get_block_epoch()
    }

    fn get_block_random_seed(&self, pointer: MemPtr) {
        VMHooksDefault.get_block_random_seed(pointer)
    }

    fn get_state_root_hash(&self, pointer: MemPtr) {
        VMHooksDefault.get_state_root_hash(pointer)
    }

    fn get_prev_block_timestamp(&self) -> i64 {
        VMHooksDefault.get_prev_block_timestamp()
    }

    fn get_prev_block_nonce(&self) -> i64 {
        VMHooksDefault.get_prev_block_nonce()
    }

    fn get_prev_block_round(&self) -> i64 {
        VMHooksDefault.get_prev_block_round()
    }

    fn get_prev_block_epoch(&self) -> i64 {
        VMHooksDefault.get_prev_block_epoch()
    }

    fn get_prev_block_random_seed(&self, pointer: MemPtr) {
        VMHooksDefault.get_prev_block_random_seed(pointer)
    }

    fn finish(&self, pointer: MemPtr, length: MemLength) {
        VMHooksDefault.finish(pointer, length)
    }

    fn execute_on_same_context(&self, gas_limit: i64, address_offset: MemPtr, value_offset: MemPtr, function_offset: MemPtr, function_length: MemLength, num_arguments: i32, arguments_length_offset: MemPtr, data_offset: MemPtr) -> i32 {
        VMHooksDefault.execute_on_same_context(gas_limit, address_offset, value_offset, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn execute_on_dest_context(&self, gas_limit: i64, address_offset: MemPtr, value_offset: MemPtr, function_offset: MemPtr, function_length: MemLength, num_arguments: i32, arguments_length_offset: MemPtr, data_offset: MemPtr) -> i32 {
        VMHooksDefault.execute_on_dest_context(gas_limit, address_offset, value_offset, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn execute_read_only(&self, gas_limit: i64, address_offset: MemPtr, function_offset: MemPtr, function_length: MemLength, num_arguments: i32, arguments_length_offset: MemPtr, data_offset: MemPtr) -> i32 {
        VMHooksDefault.execute_read_only(gas_limit, address_offset, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn create_contract(&self, gas_limit: i64, value_offset: MemPtr, code_offset: MemPtr, code_metadata_offset: MemPtr, length: MemLength, result_offset: MemPtr, num_arguments: i32, arguments_length_offset: MemPtr, data_offset: MemPtr) -> i32 {
        VMHooksDefault.create_contract(gas_limit, value_offset, code_offset, code_metadata_offset, length, result_offset, num_arguments, arguments_length_offset, data_offset)
    }

    fn deploy_from_source_contract(&self, gas_limit: i64, value_offset: MemPtr, source_contract_address_offset: MemPtr, code_metadata_offset: MemPtr, result_address_offset: MemPtr, num_arguments: i32, arguments_length_offset: MemPtr, data_offset: MemPtr) -> i32 {
        VMHooksDefault.deploy_from_source_contract(gas_limit, value_offset, source_contract_address_offset, code_metadata_offset, result_address_offset, num_arguments, arguments_length_offset, data_offset)
    }

    fn get_num_return_data(&self) -> i32 {
        VMHooksDefault.get_num_return_data()
    }

    fn get_return_data_size(&self, result_id: i32) -> i32 {
        VMHooksDefault.get_return_data_size(result_id)
    }

    fn get_return_data(&self, result_id: i32, data_offset: MemPtr) -> i32 {
        VMHooksDefault.get_return_data(result_id, data_offset)
    }

    fn clean_return_data(&self) {
        VMHooksDefault.clean_return_data()
    }

    fn delete_from_return_data(&self, result_id: i32) {
        VMHooksDefault.delete_from_return_data(result_id)
    }

    fn get_original_tx_hash(&self, data_offset: MemPtr) {
        VMHooksDefault.get_original_tx_hash(data_offset)
    }

    fn get_current_tx_hash(&self, data_offset: MemPtr) {
        VMHooksDefault.get_current_tx_hash(data_offset)
    }

    fn managed_sc_address(&self, destination_handle: i32) {
        VMHooksDefault.managed_sc_address(destination_handle)
    }

    fn managed_owner_address(&self, destination_handle: i32) {
        VMHooksDefault.managed_owner_address(destination_handle)
    }

    fn managed_caller(&self, destination_handle: i32) {
        VMHooksDefault.managed_caller(destination_handle)
    }

    fn managed_signal_error(&self, err_handle: i32) {
        VMHooksDefault.managed_signal_error(err_handle)
    }

    fn managed_write_log(&self, topics_handle: i32, data_handle: i32) {
        VMHooksDefault.managed_write_log(topics_handle, data_handle)
    }

    fn managed_get_original_tx_hash(&self, result_handle: i32) {
        VMHooksDefault.managed_get_original_tx_hash(result_handle)
    }

    fn managed_get_state_root_hash(&self, result_handle: i32) {
        VMHooksDefault.managed_get_state_root_hash(result_handle)
    }

    fn managed_get_block_random_seed(&self, result_handle: i32) {
        VMHooksDefault.managed_get_block_random_seed(result_handle)
    }

    fn managed_get_prev_block_random_seed(&self, result_handle: i32) {
        VMHooksDefault.managed_get_prev_block_random_seed(result_handle)
    }

    fn managed_get_return_data(&self, result_id: i32, result_handle: i32) {
        VMHooksDefault.managed_get_return_data(result_id, result_handle)
    }

    fn managed_get_kda_call_value(&self, kda_call_value_handle: i32, kda_handle: i32) {
        VMHooksDefault.managed_get_kda_call_value(kda_call_value_handle, kda_handle)
    }

    fn managed_get_multi_kda_call_value(&self, multi_call_value_handle: i32) {
        VMHooksDefault.managed_get_multi_kda_call_value(multi_call_value_handle)
    }

    fn managed_get_multi_kda_without_klv_call_value(&self, multi_call_value_handle: i32) {
        VMHooksDefault.managed_get_multi_kda_without_klv_call_value(multi_call_value_handle)
    }

    fn managed_get_back_transfers(&self, kda_transfers_value_handle: i32, call_value_handle: i32) {
        VMHooksDefault.managed_get_back_transfers(kda_transfers_value_handle, call_value_handle)
    }

    fn managed_get_kda_balance(&self, address_handle: i32, token_id_handle: i32, nonce: i64, value_handle: i32) {
        VMHooksDefault.managed_get_kda_balance(address_handle, token_id_handle, nonce, value_handle)
    }

    fn managed_get_user_kda(&self, address_handle: i32, ticker_handle: i32, nonce: i64, balance_handle: i32, frozen_handle: i32, last_claim_handle: i32, buckets_handle: i32, mime_handle: i32, metadata_handle: i32) {
        VMHooksDefault.managed_get_user_kda(address_handle, ticker_handle, nonce, balance_handle, frozen_handle, last_claim_handle, buckets_handle, mime_handle, metadata_handle)
    }

    fn managed_get_kda_token_data(&self, address_handle: i32, ticker_handle: i32, nonce: i64, precision_handle: i32, id_handle: i32, name_handle: i32, creator_handle: i32, admin_handle: i32, logo_handle: i32, uris_handle: i32, initial_supply_handle: i32, circulating_supply_handle: i32, max_supply_handle: i32, minted_handle: i32, burned_handle: i32, royalties_handle: i32, properties_handle: i32, attributes_handle: i32, roles_handle: i32, issue_date_handle: i32) {
        VMHooksDefault.managed_get_kda_token_data(address_handle, ticker_handle, nonce, precision_handle, id_handle, name_handle, creator_handle, admin_handle, logo_handle, uris_handle, initial_supply_handle, circulating_supply_handle, max_supply_handle, minted_handle, burned_handle, royalties_handle, properties_handle, attributes_handle, roles_handle, issue_date_handle)
    }

    fn managed_get_kda_roles(&self, ticker_handle: i32, roles_handle: i32) {
        VMHooksDefault.managed_get_kda_roles(ticker_handle, roles_handle)
    }

    fn managed_upgrade_from_source_contract(&self, dest_handle: i32, gas: i64, value_handle: i32, address_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_handle: i32) {
        VMHooksDefault.managed_upgrade_from_source_contract(dest_handle, gas, value_handle, address_handle, code_metadata_handle, arguments_handle, result_handle)
    }

    fn managed_upgrade_contract(&self, dest_handle: i32, gas: i64, value_handle: i32, code_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_handle: i32) {
        VMHooksDefault.managed_upgrade_contract(dest_handle, gas, value_handle, code_handle, code_metadata_handle, arguments_handle, result_handle)
    }

    fn managed_delete_contract(&self, dest_handle: i32, gas_limit: i64, arguments_handle: i32) {
        VMHooksDefault.managed_delete_contract(dest_handle, gas_limit, arguments_handle)
    }

    fn managed_deploy_from_source_contract(&self, gas: i64, value_handle: i32, address_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_address_handle: i32, result_handle: i32) -> i32 {
        VMHooksDefault.managed_deploy_from_source_contract(gas, value_handle, address_handle, code_metadata_handle, arguments_handle, result_address_handle, result_handle)
    }

    fn managed_create_contract(&self, gas: i64, value_handle: i32, code_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_address_handle: i32, result_handle: i32) -> i32 {
        VMHooksDefault.managed_create_contract(gas, value_handle, code_handle, code_metadata_handle, arguments_handle, result_address_handle, result_handle)
    }

    fn managed_execute_read_only(&self, gas: i64, address_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32 {
        VMHooksDefault.managed_execute_read_only(gas, address_handle, function_handle, arguments_handle, result_handle)
    }

    fn managed_execute_on_same_context(&self, gas: i64, address_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32 {
        VMHooksDefault.managed_execute_on_same_context(gas, address_handle, value_handle, function_handle, arguments_handle, result_handle)
    }

    fn managed_execute_on_dest_context(&self, gas: i64, address_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32 {
        VMHooksDefault.managed_execute_on_dest_context(gas, address_handle, value_handle, function_handle, arguments_handle, result_handle)
    }

    fn managed_multi_transfer_kda_nft_execute(&self, dst_handle: i32, token_transfers_handle: i32, gas_limit: i64, function_handle: i32, arguments_handle: i32) -> i32 {
        VMHooksDefault.managed_multi_transfer_kda_nft_execute(dst_handle, token_transfers_handle, gas_limit, function_handle, arguments_handle)
    }

    fn managed_buffer_to_hex(&self, source_handle: i32, dest_handle: i32) {
        VMHooksDefault.managed_buffer_to_hex(source_handle, dest_handle)
    }

    fn managed_get_code_metadata(&self, address_handle: i32, response_handle: i32) {
        VMHooksDefault.managed_get_code_metadata(address_handle, response_handle)
    }

    fn managed_is_builtin_function(&self, function_name_handle: i32) -> i32 {
        VMHooksDefault.managed_is_builtin_function(function_name_handle)
    }

    fn managed_get_sft_metadata(&self, ticker_handle: i32, nonce: i64, data_handle: i32) {
        VMHooksDefault.managed_get_sft_metadata(ticker_handle, nonce, data_handle)
    }

    fn managed_acc_has_perm(&self, ops: i64, source_acc_addr: i32, target_acc_addr: i32) -> i32 {
        VMHooksDefault.managed_acc_has_perm(ops, source_acc_addr, target_acc_addr)
    }

    fn big_float_new_from_parts(&self, integral_part: i32, fractional_part: i32, exponent: i32) -> i32 {
        VMHooksDefault.big_float_new_from_parts(integral_part, fractional_part, exponent)
    }

    fn big_float_new_from_frac(&self, numerator: i64, denominator: i64) -> i32 {
        VMHooksDefault.big_float_new_from_frac(numerator, denominator)
    }

    fn big_float_new_from_sci(&self, significand: i64, exponent: i64) -> i32 {
        VMHooksDefault.big_float_new_from_sci(significand, exponent)
    }

    fn big_float_add(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_float_add(destination_handle, op1_handle, op2_handle)
    }

    fn big_float_sub(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_float_sub(destination_handle, op1_handle, op2_handle)
    }

    fn big_float_mul(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_float_mul(destination_handle, op1_handle, op2_handle)
    }

    fn big_float_div(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_float_div(destination_handle, op1_handle, op2_handle)
    }

    fn big_float_neg(&self, destination_handle: i32, op_handle: i32) {
        VMHooksDefault.big_float_neg(destination_handle, op_handle)
    }

    fn big_float_clone(&self, destination_handle: i32, op_handle: i32) {
        VMHooksDefault.big_float_clone(destination_handle, op_handle)
    }

    fn big_float_cmp(&self, op1_handle: i32, op2_handle: i32) -> i32 {
        VMHooksDefault.big_float_cmp(op1_handle, op2_handle)
    }

    fn big_float_abs(&self, destination_handle: i32, op_handle: i32) {
        VMHooksDefault.big_float_abs(destination_handle, op_handle)
    }

    fn big_float_sign(&self, op_handle: i32) -> i32 {
        VMHooksDefault.big_float_sign(op_handle)
    }

    fn big_float_sqrt(&self, destination_handle: i32, op_handle: i32) {
        VMHooksDefault.big_float_sqrt(destination_handle, op_handle)
    }

    fn big_float_pow(&self, destination_handle: i32, op_handle: i32, exponent: i32) {
        VMHooksDefault.big_float_pow(destination_handle, op_handle, exponent)
    }

    fn big_float_floor(&self, dest_big_int_handle: i32, op_handle: i32) {
        VMHooksDefault.big_float_floor(dest_big_int_handle, op_handle)
    }

    fn big_float_ceil(&self, dest_big_int_handle: i32, op_handle: i32) {
        VMHooksDefault.big_float_ceil(dest_big_int_handle, op_handle)
    }

    fn big_float_truncate(&self, dest_big_int_handle: i32, op_handle: i32) {
        VMHooksDefault.big_float_truncate(dest_big_int_handle, op_handle)
    }

    fn big_float_set_int64(&self, destination_handle: i32, value: i64) {
        VMHooksDefault.big_float_set_int64(destination_handle, value)
    }

    fn big_float_is_int(&self, op_handle: i32) -> i32 {
        VMHooksDefault.big_float_is_int(op_handle)
    }

    fn big_float_set_big_int(&self, destination_handle: i32, big_int_handle: i32) {
        VMHooksDefault.big_float_set_big_int(destination_handle, big_int_handle)
    }

    fn big_float_get_const_pi(&self, destination_handle: i32) {
        VMHooksDefault.big_float_get_const_pi(destination_handle)
    }

    fn big_float_get_const_e(&self, destination_handle: i32) {
        VMHooksDefault.big_float_get_const_e(destination_handle)
    }

    fn big_int_get_unsigned_argument(&self, id: i32, destination_handle: i32) {
        VMHooksDefault.big_int_get_unsigned_argument(id, destination_handle)
    }

    fn big_int_get_signed_argument(&self, id: i32, destination_handle: i32) {
        VMHooksDefault.big_int_get_signed_argument(id, destination_handle)
    }

    fn big_int_storage_store_unsigned(&self, key_offset: MemPtr, key_length: MemLength, source_handle: i32) -> i32 {
        VMHooksDefault.big_int_storage_store_unsigned(key_offset, key_length, source_handle)
    }

    fn big_int_storage_load_unsigned(&self, key_offset: MemPtr, key_length: MemLength, destination_handle: i32) -> i32 {
        VMHooksDefault.big_int_storage_load_unsigned(key_offset, key_length, destination_handle)
    }

    fn big_int_get_call_value(&self, destination_handle: i32) {
        VMHooksDefault.big_int_get_call_value(destination_handle)
    }

    fn big_int_get_kda_call_value(&self, destination: i32) {
        VMHooksDefault.big_int_get_kda_call_value(destination)
    }

    fn big_int_get_kda_call_value_by_index(&self, destination_handle: i32, index: i32) {
        VMHooksDefault.big_int_get_kda_call_value_by_index(destination_handle, index)
    }

    fn big_int_get_external_balance(&self, address_offset: MemPtr, result: i32) {
        VMHooksDefault.big_int_get_external_balance(address_offset, result)
    }

    fn big_int_get_kda_external_balance(&self, address_offset: MemPtr, token_id_offset: MemPtr, token_id_len: MemLength, nonce: i64, result_handle: i32) {
        VMHooksDefault.big_int_get_kda_external_balance(address_offset, token_id_offset, token_id_len, nonce, result_handle)
    }

    fn big_int_new(&self, small_value: i64) -> i32 {
        VMHooksDefault.big_int_new(small_value)
    }

    fn big_int_unsigned_byte_length(&self, reference_handle: i32) -> i32 {
        VMHooksDefault.big_int_unsigned_byte_length(reference_handle)
    }

    fn big_int_signed_byte_length(&self, reference_handle: i32) -> i32 {
        VMHooksDefault.big_int_signed_byte_length(reference_handle)
    }

    fn big_int_get_unsigned_bytes(&self, reference_handle: i32, byte_offset: MemPtr) -> i32 {
        VMHooksDefault.big_int_get_unsigned_bytes(reference_handle, byte_offset)
    }

    fn big_int_get_signed_bytes(&self, reference_handle: i32, byte_offset: MemPtr) -> i32 {
        VMHooksDefault.big_int_get_signed_bytes(reference_handle, byte_offset)
    }

    fn big_int_set_unsigned_bytes(&self, destination_handle: i32, byte_offset: MemPtr, byte_length: MemLength) {
        VMHooksDefault.big_int_set_unsigned_bytes(destination_handle, byte_offset, byte_length)
    }

    fn big_int_set_signed_bytes(&self, destination_handle: i32, byte_offset: MemPtr, byte_length: MemLength) {
        VMHooksDefault.big_int_set_signed_bytes(destination_handle, byte_offset, byte_length)
    }

    fn big_int_is_int64(&self, destination_handle: i32) -> i32 {
        VMHooksDefault.big_int_is_int64(destination_handle)
    }

    fn big_int_get_int64(&self, destination_handle: i32) -> i64 {
        VMHooksDefault.big_int_get_int64(destination_handle)
    }

    fn big_int_set_int64(&self, destination_handle: i32, value: i64) {
        VMHooksDefault.big_int_set_int64(destination_handle, value)
    }

    fn big_int_add(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_add(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_sub(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_sub(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_mul(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_mul(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_tdiv(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_tdiv(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_tmod(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_tmod(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_ediv(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_ediv(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_emod(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_emod(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_sqrt(&self, destination_handle: i32, op_handle: i32) {
        VMHooksDefault.big_int_sqrt(destination_handle, op_handle)
    }

    fn big_int_pow(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_pow(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_log2(&self, op1_handle: i32) -> i32 {
        VMHooksDefault.big_int_log2(op1_handle)
    }

    fn big_int_abs(&self, destination_handle: i32, op_handle: i32) {
        VMHooksDefault.big_int_abs(destination_handle, op_handle)
    }

    fn big_int_neg(&self, destination_handle: i32, op_handle: i32) {
        VMHooksDefault.big_int_neg(destination_handle, op_handle)
    }

    fn big_int_sign(&self, op_handle: i32) -> i32 {
        VMHooksDefault.big_int_sign(op_handle)
    }

    fn big_int_cmp(&self, op1_handle: i32, op2_handle: i32) -> i32 {
        VMHooksDefault.big_int_cmp(op1_handle, op2_handle)
    }

    fn big_int_not(&self, destination_handle: i32, op_handle: i32) {
        VMHooksDefault.big_int_not(destination_handle, op_handle)
    }

    fn big_int_and(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_and(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_or(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_or(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_xor(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        VMHooksDefault.big_int_xor(destination_handle, op1_handle, op2_handle)
    }

    fn big_int_shr(&self, destination_handle: i32, op_handle: i32, bits: i32) {
        VMHooksDefault.big_int_shr(destination_handle, op_handle, bits)
    }

    fn big_int_shl(&self, destination_handle: i32, op_handle: i32, bits: i32) {
        VMHooksDefault.big_int_shl(destination_handle, op_handle, bits)
    }

    fn big_int_finish_unsigned(&self, reference_handle: i32) {
        VMHooksDefault.big_int_finish_unsigned(reference_handle)
    }

    fn big_int_finish_signed(&self, reference_handle: i32) {
        VMHooksDefault.big_int_finish_signed(reference_handle)
    }

    fn big_int_to_string(&self, big_int_handle: i32, destination_handle: i32) {
        VMHooksDefault.big_int_to_string(big_int_handle, destination_handle)
    }

    fn mbuffer_new(&self) -> i32 {
        VMHooksDefault.mbuffer_new()
    }

    fn mbuffer_new_from_bytes(&self, data_offset: MemPtr, data_length: MemLength) -> i32 {
        VMHooksDefault.mbuffer_new_from_bytes(data_offset, data_length)
    }

    fn mbuffer_get_length(&self, m_buffer_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_get_length(m_buffer_handle)
    }

    fn mbuffer_get_bytes(&self, m_buffer_handle: i32, result_offset: MemPtr) -> i32 {
        VMHooksDefault.mbuffer_get_bytes(m_buffer_handle, result_offset)
    }

    fn mbuffer_get_byte_slice(&self, source_handle: i32, starting_position: i32, slice_length: i32, result_offset: MemPtr) -> i32 {
        VMHooksDefault.mbuffer_get_byte_slice(source_handle, starting_position, slice_length, result_offset)
    }

    fn mbuffer_copy_byte_slice(&self, source_handle: i32, starting_position: i32, slice_length: i32, destination_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_copy_byte_slice(source_handle, starting_position, slice_length, destination_handle)
    }

    fn mbuffer_eq(&self, m_buffer_handle1: i32, m_buffer_handle2: i32) -> i32 {
        VMHooksDefault.mbuffer_eq(m_buffer_handle1, m_buffer_handle2)
    }

    fn mbuffer_set_bytes(&self, m_buffer_handle: i32, data_offset: MemPtr, data_length: MemLength) -> i32 {
        VMHooksDefault.mbuffer_set_bytes(m_buffer_handle, data_offset, data_length)
    }

    fn mbuffer_set_byte_slice(&self, m_buffer_handle: i32, starting_position: i32, data_length: MemLength, data_offset: MemPtr) -> i32 {
        VMHooksDefault.mbuffer_set_byte_slice(m_buffer_handle, starting_position, data_length, data_offset)
    }

    fn mbuffer_append(&self, accumulator_handle: i32, data_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_append(accumulator_handle, data_handle)
    }

    fn mbuffer_append_bytes(&self, accumulator_handle: i32, data_offset: MemPtr, data_length: MemLength) -> i32 {
        VMHooksDefault.mbuffer_append_bytes(accumulator_handle, data_offset, data_length)
    }

    fn mbuffer_to_big_int_unsigned(&self, m_buffer_handle: i32, big_int_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_to_big_int_unsigned(m_buffer_handle, big_int_handle)
    }

    fn mbuffer_to_big_int_signed(&self, m_buffer_handle: i32, big_int_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_to_big_int_signed(m_buffer_handle, big_int_handle)
    }

    fn mbuffer_from_big_int_unsigned(&self, m_buffer_handle: i32, big_int_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_from_big_int_unsigned(m_buffer_handle, big_int_handle)
    }

    fn mbuffer_from_big_int_signed(&self, m_buffer_handle: i32, big_int_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_from_big_int_signed(m_buffer_handle, big_int_handle)
    }

    fn mbuffer_to_big_float(&self, m_buffer_handle: i32, big_float_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_to_big_float(m_buffer_handle, big_float_handle)
    }

    fn mbuffer_from_big_float(&self, m_buffer_handle: i32, big_float_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_from_big_float(m_buffer_handle, big_float_handle)
    }

    fn mbuffer_storage_store(&self, key_handle: i32, source_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_storage_store(key_handle, source_handle)
    }

    fn mbuffer_storage_load(&self, key_handle: i32, destination_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_storage_load(key_handle, destination_handle)
    }

    fn mbuffer_storage_load_from_address(&self, address_handle: i32, key_handle: i32, destination_handle: i32) {
        VMHooksDefault.mbuffer_storage_load_from_address(address_handle, key_handle, destination_handle)
    }

    fn mbuffer_get_argument(&self, id: i32, destination_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_get_argument(id, destination_handle)
    }

    fn mbuffer_finish(&self, source_handle: i32) -> i32 {
        VMHooksDefault.mbuffer_finish(source_handle)
    }

    fn mbuffer_set_random(&self, destination_handle: i32, length: i32) -> i32 {
        VMHooksDefault.mbuffer_set_random(destination_handle, length)
    }

    fn managed_map_new(&self) -> i32 {
        VMHooksDefault.managed_map_new()
    }

    fn managed_map_put(&self, m_map_handle: i32, key_handle: i32, value_handle: i32) -> i32 {
        VMHooksDefault.managed_map_put(m_map_handle, key_handle, value_handle)
    }

    fn managed_map_get(&self, m_map_handle: i32, key_handle: i32, out_value_handle: i32) -> i32 {
        VMHooksDefault.managed_map_get(m_map_handle, key_handle, out_value_handle)
    }

    fn managed_map_remove(&self, m_map_handle: i32, key_handle: i32, out_value_handle: i32) -> i32 {
        VMHooksDefault.managed_map_remove(m_map_handle, key_handle, out_value_handle)
    }

    fn managed_map_contains(&self, m_map_handle: i32, key_handle: i32) -> i32 {
        VMHooksDefault.managed_map_contains(m_map_handle, key_handle)
    }

    fn small_int_get_unsigned_argument(&self, id: i32) -> i64 {
        VMHooksDefault.small_int_get_unsigned_argument(id)
    }

    fn small_int_get_signed_argument(&self, id: i32) -> i64 {
        VMHooksDefault.small_int_get_signed_argument(id)
    }

    fn small_int_finish_unsigned(&self, value: i64) {
        VMHooksDefault.small_int_finish_unsigned(value)
    }

    fn small_int_finish_signed(&self, value: i64) {
        VMHooksDefault.small_int_finish_signed(value)
    }

    fn small_int_storage_store_unsigned(&self, key_offset: MemPtr, key_length: MemLength, value: i64) -> i32 {
        VMHooksDefault.small_int_storage_store_unsigned(key_offset, key_length, value)
    }

    fn small_int_storage_store_signed(&self, key_offset: MemPtr, key_length: MemLength, value: i64) -> i32 {
        VMHooksDefault.small_int_storage_store_signed(key_offset, key_length, value)
    }

    fn small_int_storage_load_unsigned(&self, key_offset: MemPtr, key_length: MemLength) -> i64 {
        VMHooksDefault.small_int_storage_load_unsigned(key_offset, key_length)
    }

    fn small_int_storage_load_signed(&self, key_offset: MemPtr, key_length: MemLength) -> i64 {
        VMHooksDefault.small_int_storage_load_signed(key_offset, key_length)
    }

    fn int64get_argument(&self, id: i32) -> i64 {
        VMHooksDefault.int64get_argument(id)
    }

    fn int64finish(&self, value: i64) {
        VMHooksDefault.int64finish(value)
    }

    fn int64storage_store(&self, key_offset: MemPtr, key_length: MemLength, value: i64) -> i32 {
        VMHooksDefault.int64storage_store(key_offset, key_length, value)
    }

    fn int64storage_load(&self, key_offset: MemPtr, key_length: MemLength) -> i64 {
        VMHooksDefault.int64storage_load(key_offset, key_length)
    }

    fn sha256(&self, data_offset: MemPtr, length: MemLength, result_offset: MemPtr) -> i32 {
        VMHooksDefault.sha256(data_offset, length, result_offset)
    }

    fn managed_sha256(&self, input_handle: i32, output_handle: i32) -> i32 {
        VMHooksDefault.managed_sha256(input_handle, output_handle)
    }

    fn keccak256(&self, data_offset: MemPtr, length: MemLength, result_offset: MemPtr) -> i32 {
        VMHooksDefault.keccak256(data_offset, length, result_offset)
    }

    fn managed_keccak256(&self, input_handle: i32, output_handle: i32) -> i32 {
        VMHooksDefault.managed_keccak256(input_handle, output_handle)
    }

    fn ripemd160(&self, data_offset: MemPtr, length: MemLength, result_offset: MemPtr) -> i32 {
        VMHooksDefault.ripemd160(data_offset, length, result_offset)
    }

    fn managed_ripemd160(&self, input_handle: i32, output_handle: i32) -> i32 {
        VMHooksDefault.managed_ripemd160(input_handle, output_handle)
    }

    fn verify_bls(&self, key_offset: MemPtr, message_offset: MemPtr, message_length: MemLength, sig_offset: MemPtr) -> i32 {
        VMHooksDefault.verify_bls(key_offset, message_offset, message_length, sig_offset)
    }

    fn managed_verify_bls(&self, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32 {
        VMHooksDefault.managed_verify_bls(key_handle, message_handle, sig_handle)
    }

    fn verify_ed25519(&self, key_offset: MemPtr, message_offset: MemPtr, message_length: MemLength, sig_offset: MemPtr) -> i32 {
        VMHooksDefault.verify_ed25519(key_offset, message_offset, message_length, sig_offset)
    }

    fn managed_verify_ed25519(&self, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32 {
        VMHooksDefault.managed_verify_ed25519(key_handle, message_handle, sig_handle)
    }

    fn verify_custom_secp256k1(&self, key_offset: MemPtr, key_length: MemLength, message_offset: MemPtr, message_length: MemLength, sig_offset: MemPtr, hash_type: i32) -> i32 {
        VMHooksDefault.verify_custom_secp256k1(key_offset, key_length, message_offset, message_length, sig_offset, hash_type)
    }

    fn managed_verify_custom_secp256k1(&self, key_handle: i32, message_handle: i32, sig_handle: i32, hash_type: i32) -> i32 {
        VMHooksDefault.managed_verify_custom_secp256k1(key_handle, message_handle, sig_handle, hash_type)
    }

    fn verify_secp256k1(&self, key_offset: MemPtr, key_length: MemLength, message_offset: MemPtr, message_length: MemLength, sig_offset: MemPtr) -> i32 {
        VMHooksDefault.verify_secp256k1(key_offset, key_length, message_offset, message_length, sig_offset)
    }

    fn managed_verify_secp256k1(&self, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32 {
        VMHooksDefault.managed_verify_secp256k1(key_handle, message_handle, sig_handle)
    }

    fn encode_secp256k1_der_signature(&self, r_offset: MemPtr, r_length: MemLength, s_offset: MemPtr, s_length: MemLength, sig_offset: MemPtr) -> i32 {
        VMHooksDefault.encode_secp256k1_der_signature(r_offset, r_length, s_offset, s_length, sig_offset)
    }

    fn managed_encode_secp256k1_der_signature(&self, r_handle: i32, s_handle: i32, sig_handle: i32) -> i32 {
        VMHooksDefault.managed_encode_secp256k1_der_signature(r_handle, s_handle, sig_handle)
    }

    fn add_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, fst_point_xhandle: i32, fst_point_yhandle: i32, snd_point_xhandle: i32, snd_point_yhandle: i32) {
        VMHooksDefault.add_ec(x_result_handle, y_result_handle, ec_handle, fst_point_xhandle, fst_point_yhandle, snd_point_xhandle, snd_point_yhandle)
    }

    fn double_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32) {
        VMHooksDefault.double_ec(x_result_handle, y_result_handle, ec_handle, point_xhandle, point_yhandle)
    }

    fn is_on_curve_ec(&self, ec_handle: i32, point_xhandle: i32, point_yhandle: i32) -> i32 {
        VMHooksDefault.is_on_curve_ec(ec_handle, point_xhandle, point_yhandle)
    }

    fn scalar_base_mult_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: MemPtr, length: MemLength) -> i32 {
        VMHooksDefault.scalar_base_mult_ec(x_result_handle, y_result_handle, ec_handle, data_offset, length)
    }

    fn managed_scalar_base_mult_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32 {
        VMHooksDefault.managed_scalar_base_mult_ec(x_result_handle, y_result_handle, ec_handle, data_handle)
    }

    fn scalar_mult_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32, data_offset: MemPtr, length: MemLength) -> i32 {
        VMHooksDefault.scalar_mult_ec(x_result_handle, y_result_handle, ec_handle, point_xhandle, point_yhandle, data_offset, length)
    }

    fn managed_scalar_mult_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32, data_handle: i32) -> i32 {
        VMHooksDefault.managed_scalar_mult_ec(x_result_handle, y_result_handle, ec_handle, point_xhandle, point_yhandle, data_handle)
    }

    fn marshal_ec(&self, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_offset: MemPtr) -> i32 {
        VMHooksDefault.marshal_ec(x_pair_handle, y_pair_handle, ec_handle, result_offset)
    }

    fn managed_marshal_ec(&self, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_handle: i32) -> i32 {
        VMHooksDefault.managed_marshal_ec(x_pair_handle, y_pair_handle, ec_handle, result_handle)
    }

    fn marshal_compressed_ec(&self, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_offset: MemPtr) -> i32 {
        VMHooksDefault.marshal_compressed_ec(x_pair_handle, y_pair_handle, ec_handle, result_offset)
    }

    fn managed_marshal_compressed_ec(&self, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_handle: i32) -> i32 {
        VMHooksDefault.managed_marshal_compressed_ec(x_pair_handle, y_pair_handle, ec_handle, result_handle)
    }

    fn unmarshal_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: MemPtr, length: MemLength) -> i32 {
        VMHooksDefault.unmarshal_ec(x_result_handle, y_result_handle, ec_handle, data_offset, length)
    }

    fn managed_unmarshal_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32 {
        VMHooksDefault.managed_unmarshal_ec(x_result_handle, y_result_handle, ec_handle, data_handle)
    }

    fn unmarshal_compressed_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: MemPtr, length: MemLength) -> i32 {
        VMHooksDefault.unmarshal_compressed_ec(x_result_handle, y_result_handle, ec_handle, data_offset, length)
    }

    fn managed_unmarshal_compressed_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32 {
        VMHooksDefault.managed_unmarshal_compressed_ec(x_result_handle, y_result_handle, ec_handle, data_handle)
    }

    fn generate_key_ec(&self, x_pub_key_handle: i32, y_pub_key_handle: i32, ec_handle: i32, result_offset: MemPtr) -> i32 {
        VMHooksDefault.generate_key_ec(x_pub_key_handle, y_pub_key_handle, ec_handle, result_offset)
    }

    fn managed_generate_key_ec(&self, x_pub_key_handle: i32, y_pub_key_handle: i32, ec_handle: i32, result_handle: i32) -> i32 {
        VMHooksDefault.managed_generate_key_ec(x_pub_key_handle, y_pub_key_handle, ec_handle, result_handle)
    }

    fn create_ec(&self, data_offset: MemPtr, data_length: MemLength) -> i32 {
        VMHooksDefault.create_ec(data_offset, data_length)
    }

    fn managed_create_ec(&self, data_handle: i32) -> i32 {
        VMHooksDefault.managed_create_ec(data_handle)
    }

    fn get_curve_length_ec(&self, ec_handle: i32) -> i32 {
        VMHooksDefault.get_curve_length_ec(ec_handle)
    }

    fn get_priv_key_byte_length_ec(&self, ec_handle: i32) -> i32 {
        VMHooksDefault.get_priv_key_byte_length_ec(ec_handle)
    }

    fn elliptic_curve_get_values(&self, ec_handle: i32, field_order_handle: i32, base_point_order_handle: i32, eq_constant_handle: i32, x_base_point_handle: i32, y_base_point_handle: i32) -> i32 {
        VMHooksDefault.elliptic_curve_get_values(ec_handle, field_order_handle, base_point_order_handle, eq_constant_handle, x_base_point_handle, y_base_point_handle)
    }
}
