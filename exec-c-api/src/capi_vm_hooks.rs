use std::{ffi::c_void, rc::Rc, mem::transmute};

use crate::capi_vm_hook_pointers::vm_exec_vm_hook_pointers;


#[derive(Debug)]
pub struct CapiVmHooks {
    pub context_ptr: *mut c_void,
    pub pointers_ptr: vm_exec_vm_hook_pointers,
}

impl CapiVmHooks {
    pub unsafe fn new(pointers_ptr: vm_exec_vm_hook_pointers) -> Self {
        Self {
            context_ptr: std::ptr::null_mut(),
            pointers_ptr,
        }
    }
}

#[rustfmt::skip]
impl elrond_exec_service::VMHooks for CapiVmHooks {
    fn set_context_ptr(&mut self, context_ptr: *mut c_void) {
        self.context_ptr = context_ptr;
    }

    fn check_no_payment(&self) {
        println!("Calling check_no_payment ... {:?} ", self.pointers_ptr.check_no_payment_func_ptr);
        let f: fn() = unsafe {std::mem::transmute(self.pointers_ptr.check_no_payment_func_ptr) };
        f();
    }

    fn get_num_arguments(&self) -> i32 {
        println!("Calling get_num_arguments ... ");
        (self.pointers_ptr.get_num_arguments_func_ptr)(self.context_ptr)
    }

    fn get_gas_left(&self) -> i64 {
        (self.pointers_ptr.get_gas_left_func_ptr)(self.context_ptr)
    }

    fn get_sc_address(&self, result_offset: i32) {
        (self.pointers_ptr.get_sc_address_func_ptr)(self.context_ptr, result_offset)
    }

    fn get_owner_address(&self, result_offset: i32) {
        (self.pointers_ptr.get_owner_address_func_ptr)(self.context_ptr, result_offset)
    }

    fn get_shard_of_address(&self, address_offset: i32) -> i32 {
        (self.pointers_ptr.get_shard_of_address_func_ptr)(self.context_ptr, address_offset)
    }

    fn is_smart_contract(&self, address_offset: i32) -> i32 {
        (self.pointers_ptr.is_smart_contract_func_ptr)(self.context_ptr, address_offset)
    }

    fn signal_error(&self, message_offset: i32, message_length: i32) {
        (self.pointers_ptr.signal_error_func_ptr)(self.context_ptr, message_offset, message_length)
    }

    fn get_external_balance(&self, address_offset: i32, result_offset: i32) {
        (self.pointers_ptr.get_external_balance_func_ptr)(self.context_ptr, address_offset, result_offset)
    }

    fn get_block_hash(&self, nonce: i64, result_offset: i32) -> i32 {
        (self.pointers_ptr.get_block_hash_func_ptr)(self.context_ptr, nonce, result_offset)
    }

    fn get_esdt_balance(&self, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64, result_offset: i32) -> i32 {
        (self.pointers_ptr.get_esdt_balance_func_ptr)(self.context_ptr, address_offset, token_id_offset, token_id_len, nonce, result_offset)
    }

    fn get_esdt_nft_name_length(&self, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64) -> i32 {
        (self.pointers_ptr.get_esdt_nft_name_length_func_ptr)(self.context_ptr, address_offset, token_id_offset, token_id_len, nonce)
    }

    fn get_esdt_nft_attribute_length(&self, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64) -> i32 {
        (self.pointers_ptr.get_esdt_nft_attribute_length_func_ptr)(self.context_ptr, address_offset, token_id_offset, token_id_len, nonce)
    }

    fn get_esdt_nft_uri_length(&self, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64) -> i32 {
        (self.pointers_ptr.get_esdt_nft_uri_length_func_ptr)(self.context_ptr, address_offset, token_id_offset, token_id_len, nonce)
    }

    fn get_esdt_token_data(&self, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64, value_handle: i32, properties_offset: i32, hash_offset: i32, name_offset: i32, attributes_offset: i32, creator_offset: i32, royalties_handle: i32, uris_offset: i32) -> i32 {
        (self.pointers_ptr.get_esdt_token_data_func_ptr)(self.context_ptr, address_offset, token_id_offset, token_id_len, nonce, value_handle, properties_offset, hash_offset, name_offset, attributes_offset, creator_offset, royalties_handle, uris_offset)
    }

    fn get_esdt_local_roles(&self, token_id_handle: i32) -> i64 {
        (self.pointers_ptr.get_esdt_local_roles_func_ptr)(self.context_ptr, token_id_handle)
    }

    fn validate_token_identifier(&self, token_id_handle: i32) -> i32 {
        (self.pointers_ptr.validate_token_identifier_func_ptr)(self.context_ptr, token_id_handle)
    }

    fn transfer_value(&self, dest_offset: i32, value_offset: i32, data_offset: i32, length: i32) -> i32 {
        (self.pointers_ptr.transfer_value_func_ptr)(self.context_ptr, dest_offset, value_offset, data_offset, length)
    }

    fn transfer_value_execute(&self, dest_offset: i32, value_offset: i32, gas_limit: i64, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.transfer_value_execute_func_ptr)(self.context_ptr, dest_offset, value_offset, gas_limit, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn transfer_esdt_execute(&self, dest_offset: i32, token_id_offset: i32, token_id_len: i32, value_offset: i32, gas_limit: i64, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.transfer_esdt_execute_func_ptr)(self.context_ptr, dest_offset, token_id_offset, token_id_len, value_offset, gas_limit, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn transfer_esdt_nft_execute(&self, dest_offset: i32, token_id_offset: i32, token_id_len: i32, value_offset: i32, nonce: i64, gas_limit: i64, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.transfer_esdt_nft_execute_func_ptr)(self.context_ptr, dest_offset, token_id_offset, token_id_len, value_offset, nonce, gas_limit, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn multi_transfer_esdt_nft_execute(&self, dest_offset: i32, num_token_transfers: i32, token_transfers_args_length_offset: i32, token_transfer_data_offset: i32, gas_limit: i64, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.multi_transfer_esdt_nft_execute_func_ptr)(self.context_ptr, dest_offset, num_token_transfers, token_transfers_args_length_offset, token_transfer_data_offset, gas_limit, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn create_async_call(&self, dest_offset: i32, value_offset: i32, data_offset: i32, data_length: i32, success_offset: i32, success_length: i32, error_offset: i32, error_length: i32, gas: i64, extra_gas_for_callback: i64) -> i32 {
        (self.pointers_ptr.create_async_call_func_ptr)(self.context_ptr, dest_offset, value_offset, data_offset, data_length, success_offset, success_length, error_offset, error_length, gas, extra_gas_for_callback)
    }

    fn set_async_context_callback(&self, callback: i32, callback_length: i32, data: i32, data_length: i32, gas: i64) -> i32 {
        (self.pointers_ptr.set_async_context_callback_func_ptr)(self.context_ptr, callback, callback_length, data, data_length, gas)
    }

    fn upgrade_contract(&self, dest_offset: i32, gas_limit: i64, value_offset: i32, code_offset: i32, code_metadata_offset: i32, length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) {
        (self.pointers_ptr.upgrade_contract_func_ptr)(self.context_ptr, dest_offset, gas_limit, value_offset, code_offset, code_metadata_offset, length, num_arguments, arguments_length_offset, data_offset)
    }

    fn upgrade_from_source_contract(&self, dest_offset: i32, gas_limit: i64, value_offset: i32, source_contract_address_offset: i32, code_metadata_offset: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) {
        (self.pointers_ptr.upgrade_from_source_contract_func_ptr)(self.context_ptr, dest_offset, gas_limit, value_offset, source_contract_address_offset, code_metadata_offset, num_arguments, arguments_length_offset, data_offset)
    }

    fn delete_contract(&self, dest_offset: i32, gas_limit: i64, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) {
        (self.pointers_ptr.delete_contract_func_ptr)(self.context_ptr, dest_offset, gas_limit, num_arguments, arguments_length_offset, data_offset)
    }

    fn async_call(&self, dest_offset: i32, value_offset: i32, data_offset: i32, length: i32) {
        (self.pointers_ptr.async_call_func_ptr)(self.context_ptr, dest_offset, value_offset, data_offset, length)
    }

    fn get_argument_length(&self, id: i32) -> i32 {
        (self.pointers_ptr.get_argument_length_func_ptr)(self.context_ptr, id)
    }

    fn get_argument(&self, id: i32, arg_offset: i32) -> i32 {
        (self.pointers_ptr.get_argument_func_ptr)(self.context_ptr, id, arg_offset)
    }

    fn get_function(&self, function_offset: i32) -> i32 {
        (self.pointers_ptr.get_function_func_ptr)(self.context_ptr, function_offset)
    }



    fn storage_store(&self, key_offset: i32, key_length: i32, data_offset: i32, data_length: i32) -> i32 {
        (self.pointers_ptr.storage_store_func_ptr)(self.context_ptr, key_offset, key_length, data_offset, data_length)
    }

    fn storage_load_length(&self, key_offset: i32, key_length: i32) -> i32 {
        (self.pointers_ptr.storage_load_length_func_ptr)(self.context_ptr, key_offset, key_length)
    }

    fn storage_load_from_address(&self, address_offset: i32, key_offset: i32, key_length: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.storage_load_from_address_func_ptr)(self.context_ptr, address_offset, key_offset, key_length, data_offset)
    }

    fn storage_load(&self, key_offset: i32, key_length: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.storage_load_func_ptr)(self.context_ptr, key_offset, key_length, data_offset)
    }

    fn set_storage_lock(&self, key_offset: i32, key_length: i32, lock_timestamp: i64) -> i32 {
        (self.pointers_ptr.set_storage_lock_func_ptr)(self.context_ptr, key_offset, key_length, lock_timestamp)
    }

    fn get_storage_lock(&self, key_offset: i32, key_length: i32) -> i64 {
        (self.pointers_ptr.get_storage_lock_func_ptr)(self.context_ptr, key_offset, key_length)
    }

    fn is_storage_locked(&self, key_offset: i32, key_length: i32) -> i32 {
        (self.pointers_ptr.is_storage_locked_func_ptr)(self.context_ptr, key_offset, key_length)
    }

    fn clear_storage_lock(&self, key_offset: i32, key_length: i32) -> i32 {
        (self.pointers_ptr.clear_storage_lock_func_ptr)(self.context_ptr, key_offset, key_length)
    }

    fn get_caller(&self, result_offset: i32) {
        (self.pointers_ptr.get_caller_func_ptr)(self.context_ptr, result_offset)
    }

    fn get_call_value(&self, result_offset: i32) -> i32 {
        (self.pointers_ptr.get_call_value_func_ptr)(self.context_ptr, result_offset)
    }

    fn get_esdt_value(&self, result_offset: i32) -> i32 {
        (self.pointers_ptr.get_esdt_value_func_ptr)(self.context_ptr, result_offset)
    }

    fn get_esdt_value_by_index(&self, result_offset: i32, index: i32) -> i32 {
        (self.pointers_ptr.get_esdt_value_by_index_func_ptr)(self.context_ptr, result_offset, index)
    }

    fn get_esdt_token_name(&self, result_offset: i32) -> i32 {
        (self.pointers_ptr.get_esdt_token_name_func_ptr)(self.context_ptr, result_offset)
    }

    fn get_esdt_token_name_by_index(&self, result_offset: i32, index: i32) -> i32 {
        (self.pointers_ptr.get_esdt_token_name_by_index_func_ptr)(self.context_ptr, result_offset, index)
    }

    fn get_esdt_token_nonce(&self) -> i64 {
        (self.pointers_ptr.get_esdt_token_nonce_func_ptr)(self.context_ptr)
    }

    fn get_esdt_token_nonce_by_index(&self, index: i32) -> i64 {
        (self.pointers_ptr.get_esdt_token_nonce_by_index_func_ptr)(self.context_ptr, index)
    }

    fn get_current_esdt_nft_nonce(&self, address_offset: i32, token_id_offset: i32, token_id_len: i32) -> i64 {
        (self.pointers_ptr.get_current_esdt_nft_nonce_func_ptr)(self.context_ptr, address_offset, token_id_offset, token_id_len)
    }

    fn get_esdt_token_type(&self) -> i32 {
        (self.pointers_ptr.get_esdt_token_type_func_ptr)(self.context_ptr)
    }

    fn get_esdt_token_type_by_index(&self, index: i32) -> i32 {
        (self.pointers_ptr.get_esdt_token_type_by_index_func_ptr)(self.context_ptr, index)
    }

    fn get_num_esdt_transfers(&self) -> i32 {
        (self.pointers_ptr.get_num_esdt_transfers_func_ptr)(self.context_ptr)
    }

    fn get_call_value_token_name(&self, call_value_offset: i32, token_name_offset: i32) -> i32 {
        (self.pointers_ptr.get_call_value_token_name_func_ptr)(self.context_ptr, call_value_offset, token_name_offset)
    }

    fn get_call_value_token_name_by_index(&self, call_value_offset: i32, token_name_offset: i32, index: i32) -> i32 {
        (self.pointers_ptr.get_call_value_token_name_by_index_func_ptr)(self.context_ptr, call_value_offset, token_name_offset, index)
    }

    fn write_log(&self, data_pointer: i32, data_length: i32, topic_ptr: i32, num_topics: i32) {
        (self.pointers_ptr.write_log_func_ptr)(self.context_ptr, data_pointer, data_length, topic_ptr, num_topics)
    }

    fn write_event_log(&self, num_topics: i32, topic_lengths_offset: i32, topic_offset: i32, data_offset: i32, data_length: i32) {
        (self.pointers_ptr.write_event_log_func_ptr)(self.context_ptr, num_topics, topic_lengths_offset, topic_offset, data_offset, data_length)
    }

    fn get_block_timestamp(&self) -> i64 {
        (self.pointers_ptr.get_block_timestamp_func_ptr)(self.context_ptr)
    }

    fn get_block_nonce(&self) -> i64 {
        (self.pointers_ptr.get_block_nonce_func_ptr)(self.context_ptr)
    }

    fn get_block_round(&self) -> i64 {
        (self.pointers_ptr.get_block_round_func_ptr)(self.context_ptr)
    }

    fn get_block_epoch(&self) -> i64 {
        (self.pointers_ptr.get_block_epoch_func_ptr)(self.context_ptr)
    }

    fn get_block_random_seed(&self, pointer: i32) {
        (self.pointers_ptr.get_block_random_seed_func_ptr)(self.context_ptr, pointer)
    }

    fn get_state_root_hash(&self, pointer: i32) {
        (self.pointers_ptr.get_state_root_hash_func_ptr)(self.context_ptr, pointer)
    }

    fn get_prev_block_timestamp(&self) -> i64 {
        (self.pointers_ptr.get_prev_block_timestamp_func_ptr)(self.context_ptr)
    }

    fn get_prev_block_nonce(&self) -> i64 {
        (self.pointers_ptr.get_prev_block_nonce_func_ptr)(self.context_ptr)
    }

    fn get_prev_block_round(&self) -> i64 {
        (self.pointers_ptr.get_prev_block_round_func_ptr)(self.context_ptr)
    }

    fn get_prev_block_epoch(&self) -> i64 {
        (self.pointers_ptr.get_prev_block_epoch_func_ptr)(self.context_ptr)
    }

    fn get_prev_block_random_seed(&self, pointer: i32) {
        (self.pointers_ptr.get_prev_block_random_seed_func_ptr)(self.context_ptr, pointer)
    }

    fn finish(&self, pointer: i32, length: i32) {
        (self.pointers_ptr.finish_func_ptr)(self.context_ptr, pointer, length)
    }

    fn execute_on_same_context(&self, gas_limit: i64, address_offset: i32, value_offset: i32, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.execute_on_same_context_func_ptr)(self.context_ptr, gas_limit, address_offset, value_offset, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn execute_on_dest_context(&self, gas_limit: i64, address_offset: i32, value_offset: i32, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.execute_on_dest_context_func_ptr)(self.context_ptr, gas_limit, address_offset, value_offset, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn execute_read_only(&self, gas_limit: i64, address_offset: i32, function_offset: i32, function_length: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.execute_read_only_func_ptr)(self.context_ptr, gas_limit, address_offset, function_offset, function_length, num_arguments, arguments_length_offset, data_offset)
    }

    fn create_contract(&self, gas_limit: i64, value_offset: i32, code_offset: i32, code_metadata_offset: i32, length: i32, result_offset: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.create_contract_func_ptr)(self.context_ptr, gas_limit, value_offset, code_offset, code_metadata_offset, length, result_offset, num_arguments, arguments_length_offset, data_offset)
    }

    fn deploy_from_source_contract(&self, gas_limit: i64, value_offset: i32, source_contract_address_offset: i32, code_metadata_offset: i32, result_address_offset: i32, num_arguments: i32, arguments_length_offset: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.deploy_from_source_contract_func_ptr)(self.context_ptr, gas_limit, value_offset, source_contract_address_offset, code_metadata_offset, result_address_offset, num_arguments, arguments_length_offset, data_offset)
    }

    fn get_num_return_data(&self) -> i32 {
        (self.pointers_ptr.get_num_return_data_func_ptr)(self.context_ptr)
    }

    fn get_return_data_size(&self, result_id: i32) -> i32 {
        (self.pointers_ptr.get_return_data_size_func_ptr)(self.context_ptr, result_id)
    }

    fn get_return_data(&self, result_id: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.get_return_data_func_ptr)(self.context_ptr, result_id, data_offset)
    }

    fn clean_return_data(&self) {
        (self.pointers_ptr.clean_return_data_func_ptr)(self.context_ptr)
    }

    fn delete_from_return_data(&self, result_id: i32) {
        (self.pointers_ptr.delete_from_return_data_func_ptr)(self.context_ptr, result_id)
    }

    fn get_original_tx_hash(&self, data_offset: i32) {
        (self.pointers_ptr.get_original_tx_hash_func_ptr)(self.context_ptr, data_offset)
    }

    fn get_current_tx_hash(&self, data_offset: i32) {
        (self.pointers_ptr.get_current_tx_hash_func_ptr)(self.context_ptr, data_offset)
    }

    fn get_prev_tx_hash(&self, data_offset: i32) {
        (self.pointers_ptr.get_prev_tx_hash_func_ptr)(self.context_ptr, data_offset)
    }

    fn managed_sc_address(&self, destination_handle: i32) {
        (self.pointers_ptr.managed_sc_address_func_ptr)(self.context_ptr, destination_handle)
    }

    fn managed_owner_address(&self, destination_handle: i32) {
        (self.pointers_ptr.managed_owner_address_func_ptr)(self.context_ptr, destination_handle)
    }

    fn managed_caller(&self, destination_handle: i32) {
        (self.pointers_ptr.managed_caller_func_ptr)(self.context_ptr, destination_handle)
    }

    fn managed_signal_error(&self, err_handle: i32) {
        (self.pointers_ptr.managed_signal_error_func_ptr)(self.context_ptr, err_handle)
    }

    fn managed_write_log(&self, topics_handle: i32, data_handle: i32) {
        (self.pointers_ptr.managed_write_log_func_ptr)(self.context_ptr, topics_handle, data_handle)
    }

    fn managed_get_original_tx_hash(&self, result_handle: i32) {
        (self.pointers_ptr.managed_get_original_tx_hash_func_ptr)(self.context_ptr, result_handle)
    }

    fn managed_get_state_root_hash(&self, result_handle: i32) {
        (self.pointers_ptr.managed_get_state_root_hash_func_ptr)(self.context_ptr, result_handle)
    }

    fn managed_get_block_random_seed(&self, result_handle: i32) {
        (self.pointers_ptr.managed_get_block_random_seed_func_ptr)(self.context_ptr, result_handle)
    }

    fn managed_get_prev_block_random_seed(&self, result_handle: i32) {
        (self.pointers_ptr.managed_get_prev_block_random_seed_func_ptr)(self.context_ptr, result_handle)
    }

    fn managed_get_return_data(&self, result_id: i32, result_handle: i32) {
        (self.pointers_ptr.managed_get_return_data_func_ptr)(self.context_ptr, result_id, result_handle)
    }

    fn managed_get_multi_esdt_call_value(&self, multi_call_value_handle: i32) {
        (self.pointers_ptr.managed_get_multi_esdt_call_value_func_ptr)(self.context_ptr, multi_call_value_handle)
    }

    fn managed_get_esdt_balance(&self, address_handle: i32, token_id_handle: i32, nonce: i64, value_handle: i32) {
        (self.pointers_ptr.managed_get_esdt_balance_func_ptr)(self.context_ptr, address_handle, token_id_handle, nonce, value_handle)
    }

    fn managed_get_esdt_token_data(&self, address_handle: i32, token_id_handle: i32, nonce: i64, value_handle: i32, properties_handle: i32, hash_handle: i32, name_handle: i32, attributes_handle: i32, creator_handle: i32, royalties_handle: i32, uris_handle: i32) {
        (self.pointers_ptr.managed_get_esdt_token_data_func_ptr)(self.context_ptr, address_handle, token_id_handle, nonce, value_handle, properties_handle, hash_handle, name_handle, attributes_handle, creator_handle, royalties_handle, uris_handle)
    }

    fn managed_async_call(&self, dest_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32) {
        (self.pointers_ptr.managed_async_call_func_ptr)(self.context_ptr, dest_handle, value_handle, function_handle, arguments_handle)
    }

    fn managed_create_async_call(&self, dest_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32, success_offset: i32, success_length: i32, error_offset: i32, error_length: i32, gas: i64, extra_gas_for_callback: i64, callback_closure_handle: i32) -> i32 {
        (self.pointers_ptr.managed_create_async_call_func_ptr)(self.context_ptr, dest_handle, value_handle, function_handle, arguments_handle, success_offset, success_length, error_offset, error_length, gas, extra_gas_for_callback, callback_closure_handle)
    }

    fn managed_get_callback_closure(&self, callback_closure_handle: i32) {
        (self.pointers_ptr.managed_get_callback_closure_func_ptr)(self.context_ptr, callback_closure_handle)
    }

    fn managed_upgrade_from_source_contract(&self, dest_handle: i32, gas: i64, value_handle: i32, address_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_handle: i32) {
        (self.pointers_ptr.managed_upgrade_from_source_contract_func_ptr)(self.context_ptr, dest_handle, gas, value_handle, address_handle, code_metadata_handle, arguments_handle, result_handle)
    }

    fn managed_upgrade_contract(&self, dest_handle: i32, gas: i64, value_handle: i32, code_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_handle: i32) {
        (self.pointers_ptr.managed_upgrade_contract_func_ptr)(self.context_ptr, dest_handle, gas, value_handle, code_handle, code_metadata_handle, arguments_handle, result_handle)
    }

    fn managed_delete_contract(&self, dest_handle: i32, gas_limit: i64, arguments_handle: i32) {
        (self.pointers_ptr.managed_delete_contract_func_ptr)(self.context_ptr, dest_handle, gas_limit, arguments_handle)
    }

    fn managed_deploy_from_source_contract(&self, gas: i64, value_handle: i32, address_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_address_handle: i32, result_handle: i32) -> i32 {
        (self.pointers_ptr.managed_deploy_from_source_contract_func_ptr)(self.context_ptr, gas, value_handle, address_handle, code_metadata_handle, arguments_handle, result_address_handle, result_handle)
    }

    fn managed_create_contract(&self, gas: i64, value_handle: i32, code_handle: i32, code_metadata_handle: i32, arguments_handle: i32, result_address_handle: i32, result_handle: i32) -> i32 {
        (self.pointers_ptr.managed_create_contract_func_ptr)(self.context_ptr, gas, value_handle, code_handle, code_metadata_handle, arguments_handle, result_address_handle, result_handle)
    }

    fn managed_execute_read_only(&self, gas: i64, address_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32 {
        (self.pointers_ptr.managed_execute_read_only_func_ptr)(self.context_ptr, gas, address_handle, function_handle, arguments_handle, result_handle)
    }

    fn managed_execute_on_same_context(&self, gas: i64, address_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32 {
        (self.pointers_ptr.managed_execute_on_same_context_func_ptr)(self.context_ptr, gas, address_handle, value_handle, function_handle, arguments_handle, result_handle)
    }

    fn managed_execute_on_dest_context(&self, gas: i64, address_handle: i32, value_handle: i32, function_handle: i32, arguments_handle: i32, result_handle: i32) -> i32 {
        (self.pointers_ptr.managed_execute_on_dest_context_func_ptr)(self.context_ptr, gas, address_handle, value_handle, function_handle, arguments_handle, result_handle)
    }

    fn managed_multi_transfer_esdt_nft_execute(&self, dst_handle: i32, token_transfers_handle: i32, gas_limit: i64, function_handle: i32, arguments_handle: i32) -> i32 {
        (self.pointers_ptr.managed_multi_transfer_esdt_nft_execute_func_ptr)(self.context_ptr, dst_handle, token_transfers_handle, gas_limit, function_handle, arguments_handle)
    }

    fn managed_transfer_value_execute(&self, dst_handle: i32, value_handle: i32, gas_limit: i64, function_handle: i32, arguments_handle: i32) -> i32 {
        (self.pointers_ptr.managed_transfer_value_execute_func_ptr)(self.context_ptr, dst_handle, value_handle, gas_limit, function_handle, arguments_handle)
    }

    fn managed_is_esdt_frozen(&self, address_handle: i32, token_id_handle: i32, nonce: i64) -> i32 {
        (self.pointers_ptr.managed_is_esdt_frozen_func_ptr)(self.context_ptr, address_handle, token_id_handle, nonce)
    }

    fn managed_is_esdt_limited_transfer(&self, token_id_handle: i32) -> i32 {
        (self.pointers_ptr.managed_is_esdt_limited_transfer_func_ptr)(self.context_ptr, token_id_handle)
    }

    fn managed_is_esdt_paused(&self, token_id_handle: i32) -> i32 {
        (self.pointers_ptr.managed_is_esdt_paused_func_ptr)(self.context_ptr, token_id_handle)
    }

    fn managed_buffer_to_hex(&self, source_handle: i32, dest_handle: i32) {
        (self.pointers_ptr.managed_buffer_to_hex_func_ptr)(self.context_ptr, source_handle, dest_handle)
    }

    fn big_float_new_from_parts(&self, integral_part: i32, fractional_part: i32, exponent: i32) -> i32 {
        (self.pointers_ptr.big_float_new_from_parts_func_ptr)(self.context_ptr, integral_part, fractional_part, exponent)
    }

    fn big_float_new_from_frac(&self, numerator: i64, denominator: i64) -> i32 {
        (self.pointers_ptr.big_float_new_from_frac_func_ptr)(self.context_ptr, numerator, denominator)
    }

    fn big_float_new_from_sci(&self, significand: i64, exponent: i64) -> i32 {
        (self.pointers_ptr.big_float_new_from_sci_func_ptr)(self.context_ptr, significand, exponent)
    }

    fn big_float_add(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_float_add_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_float_sub(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_float_sub_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_float_mul(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_float_mul_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_float_div(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_float_div_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_float_neg(&self, destination_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_float_neg_func_ptr)(self.context_ptr, destination_handle, op_handle)
    }

    fn big_float_clone(&self, destination_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_float_clone_func_ptr)(self.context_ptr, destination_handle, op_handle)
    }

    fn big_float_cmp(&self, op1_handle: i32, op2_handle: i32) -> i32 {
        (self.pointers_ptr.big_float_cmp_func_ptr)(self.context_ptr, op1_handle, op2_handle)
    }

    fn big_float_abs(&self, destination_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_float_abs_func_ptr)(self.context_ptr, destination_handle, op_handle)
    }

    fn big_float_sign(&self, op_handle: i32) -> i32 {
        (self.pointers_ptr.big_float_sign_func_ptr)(self.context_ptr, op_handle)
    }

    fn big_float_sqrt(&self, destination_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_float_sqrt_func_ptr)(self.context_ptr, destination_handle, op_handle)
    }

    fn big_float_pow(&self, destination_handle: i32, op_handle: i32, exponent: i32) {
        (self.pointers_ptr.big_float_pow_func_ptr)(self.context_ptr, destination_handle, op_handle, exponent)
    }

    fn big_float_floor(&self, dest_big_int_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_float_floor_func_ptr)(self.context_ptr, dest_big_int_handle, op_handle)
    }

    fn big_float_ceil(&self, dest_big_int_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_float_ceil_func_ptr)(self.context_ptr, dest_big_int_handle, op_handle)
    }

    fn big_float_truncate(&self, dest_big_int_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_float_truncate_func_ptr)(self.context_ptr, dest_big_int_handle, op_handle)
    }

    fn big_float_set_int64(&self, destination_handle: i32, value: i64) {
        (self.pointers_ptr.big_float_set_int64_func_ptr)(self.context_ptr, destination_handle, value)
    }

    fn big_float_is_int(&self, op_handle: i32) -> i32 {
        (self.pointers_ptr.big_float_is_int_func_ptr)(self.context_ptr, op_handle)
    }

    fn big_float_set_big_int(&self, destination_handle: i32, big_int_handle: i32) {
        (self.pointers_ptr.big_float_set_big_int_func_ptr)(self.context_ptr, destination_handle, big_int_handle)
    }

    fn big_float_get_const_pi(&self, destination_handle: i32) {
        (self.pointers_ptr.big_float_get_const_pi_func_ptr)(self.context_ptr, destination_handle)
    }

    fn big_float_get_const_e(&self, destination_handle: i32) {
        (self.pointers_ptr.big_float_get_const_e_func_ptr)(self.context_ptr, destination_handle)
    }

    fn big_int_get_unsigned_argument(&self, id: i32, destination_handle: i32) {
        (self.pointers_ptr.big_int_get_unsigned_argument_func_ptr)(self.context_ptr, id, destination_handle)
    }

    fn big_int_get_signed_argument(&self, id: i32, destination_handle: i32) {
        (self.pointers_ptr.big_int_get_signed_argument_func_ptr)(self.context_ptr, id, destination_handle)
    }

    fn big_int_storage_store_unsigned(&self, key_offset: i32, key_length: i32, source_handle: i32) -> i32 {
        (self.pointers_ptr.big_int_storage_store_unsigned_func_ptr)(self.context_ptr, key_offset, key_length, source_handle)
    }

    fn big_int_storage_load_unsigned(&self, key_offset: i32, key_length: i32, destination_handle: i32) -> i32 {
        (self.pointers_ptr.big_int_storage_load_unsigned_func_ptr)(self.context_ptr, key_offset, key_length, destination_handle)
    }

    fn big_int_get_call_value(&self, destination_handle: i32) {
        (self.pointers_ptr.big_int_get_call_value_func_ptr)(self.context_ptr, destination_handle)
    }

    fn big_int_get_esdt_call_value(&self, destination: i32) {
        (self.pointers_ptr.big_int_get_esdt_call_value_func_ptr)(self.context_ptr, destination)
    }

    fn big_int_get_esdt_call_value_by_index(&self, destination_handle: i32, index: i32) {
        (self.pointers_ptr.big_int_get_esdt_call_value_by_index_func_ptr)(self.context_ptr, destination_handle, index)
    }

    fn big_int_get_external_balance(&self, address_offset: i32, result: i32) {
        (self.pointers_ptr.big_int_get_external_balance_func_ptr)(self.context_ptr, address_offset, result)
    }

    fn big_int_get_esdt_external_balance(&self, address_offset: i32, token_id_offset: i32, token_id_len: i32, nonce: i64, result_handle: i32) {
        (self.pointers_ptr.big_int_get_esdt_external_balance_func_ptr)(self.context_ptr, address_offset, token_id_offset, token_id_len, nonce, result_handle)
    }

    fn big_int_new(&self, small_value: i64) -> i32 {
        (self.pointers_ptr.big_int_new_func_ptr)(self.context_ptr, small_value)
    }

    fn big_int_unsigned_byte_length(&self, reference_handle: i32) -> i32 {
        (self.pointers_ptr.big_int_unsigned_byte_length_func_ptr)(self.context_ptr, reference_handle)
    }

    fn big_int_signed_byte_length(&self, reference_handle: i32) -> i32 {
        (self.pointers_ptr.big_int_signed_byte_length_func_ptr)(self.context_ptr, reference_handle)
    }

    fn big_int_get_unsigned_bytes(&self, reference_handle: i32, byte_offset: i32) -> i32 {
        (self.pointers_ptr.big_int_get_unsigned_bytes_func_ptr)(self.context_ptr, reference_handle, byte_offset)
    }

    fn big_int_get_signed_bytes(&self, reference_handle: i32, byte_offset: i32) -> i32 {
        (self.pointers_ptr.big_int_get_signed_bytes_func_ptr)(self.context_ptr, reference_handle, byte_offset)
    }

    fn big_int_set_unsigned_bytes(&self, destination_handle: i32, byte_offset: i32, byte_length: i32) {
        (self.pointers_ptr.big_int_set_unsigned_bytes_func_ptr)(self.context_ptr, destination_handle, byte_offset, byte_length)
    }

    fn big_int_set_signed_bytes(&self, destination_handle: i32, byte_offset: i32, byte_length: i32) {
        (self.pointers_ptr.big_int_set_signed_bytes_func_ptr)(self.context_ptr, destination_handle, byte_offset, byte_length)
    }

    fn big_int_is_int64(&self, destination_handle: i32) -> i32 {
        (self.pointers_ptr.big_int_is_int64_func_ptr)(self.context_ptr, destination_handle)
    }

    fn big_int_get_int64(&self, destination_handle: i32) -> i64 {
        (self.pointers_ptr.big_int_get_int64_func_ptr)(self.context_ptr, destination_handle)
    }

    fn big_int_set_int64(&self, destination_handle: i32, value: i64) {
        (self.pointers_ptr.big_int_set_int64_func_ptr)(self.context_ptr, destination_handle, value)
    }

    fn big_int_add(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_add_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_sub(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_sub_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_mul(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_mul_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_tdiv(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_tdiv_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_tmod(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_tmod_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_ediv(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_ediv_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_emod(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_emod_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_sqrt(&self, destination_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_int_sqrt_func_ptr)(self.context_ptr, destination_handle, op_handle)
    }

    fn big_int_pow(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_pow_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_log2(&self, op1_handle: i32) -> i32 {
        (self.pointers_ptr.big_int_log2_func_ptr)(self.context_ptr, op1_handle)
    }

    fn big_int_abs(&self, destination_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_int_abs_func_ptr)(self.context_ptr, destination_handle, op_handle)
    }

    fn big_int_neg(&self, destination_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_int_neg_func_ptr)(self.context_ptr, destination_handle, op_handle)
    }

    fn big_int_sign(&self, op_handle: i32) -> i32 {
        (self.pointers_ptr.big_int_sign_func_ptr)(self.context_ptr, op_handle)
    }

    fn big_int_cmp(&self, op1_handle: i32, op2_handle: i32) -> i32 {
        (self.pointers_ptr.big_int_cmp_func_ptr)(self.context_ptr, op1_handle, op2_handle)
    }

    fn big_int_not(&self, destination_handle: i32, op_handle: i32) {
        (self.pointers_ptr.big_int_not_func_ptr)(self.context_ptr, destination_handle, op_handle)
    }

    fn big_int_and(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_and_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_or(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_or_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_xor(&self, destination_handle: i32, op1_handle: i32, op2_handle: i32) {
        (self.pointers_ptr.big_int_xor_func_ptr)(self.context_ptr, destination_handle, op1_handle, op2_handle)
    }

    fn big_int_shr(&self, destination_handle: i32, op_handle: i32, bits: i32) {
        (self.pointers_ptr.big_int_shr_func_ptr)(self.context_ptr, destination_handle, op_handle, bits)
    }

    fn big_int_shl(&self, destination_handle: i32, op_handle: i32, bits: i32) {
        (self.pointers_ptr.big_int_shl_func_ptr)(self.context_ptr, destination_handle, op_handle, bits)
    }

    fn big_int_finish_unsigned(&self, reference_handle: i32) {
        (self.pointers_ptr.big_int_finish_unsigned_func_ptr)(self.context_ptr, reference_handle)
    }

    fn big_int_finish_signed(&self, reference_handle: i32) {
        (self.pointers_ptr.big_int_finish_signed_func_ptr)(self.context_ptr, reference_handle)
    }

    fn big_int_to_string(&self, big_int_handle: i32, destination_handle: i32) {
        (self.pointers_ptr.big_int_to_string_func_ptr)(self.context_ptr, big_int_handle, destination_handle)
    }

    fn mbuffer_new(&self) -> i32 {
        (self.pointers_ptr.mbuffer_new_func_ptr)(self.context_ptr)
    }

    fn mbuffer_new_from_bytes(&self, data_offset: i32, data_length: i32) -> i32 {
        (self.pointers_ptr.mbuffer_new_from_bytes_func_ptr)(self.context_ptr, data_offset, data_length)
    }

    fn mbuffer_get_length(&self, m_buffer_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_get_length_func_ptr)(self.context_ptr, m_buffer_handle)
    }

    fn mbuffer_get_bytes(&self, m_buffer_handle: i32, result_offset: i32) -> i32 {
        (self.pointers_ptr.mbuffer_get_bytes_func_ptr)(self.context_ptr, m_buffer_handle, result_offset)
    }

    fn mbuffer_get_byte_slice(&self, source_handle: i32, starting_position: i32, slice_length: i32, result_offset: i32) -> i32 {
        (self.pointers_ptr.mbuffer_get_byte_slice_func_ptr)(self.context_ptr, source_handle, starting_position, slice_length, result_offset)
    }

    fn mbuffer_copy_byte_slice(&self, source_handle: i32, starting_position: i32, slice_length: i32, destination_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_copy_byte_slice_func_ptr)(self.context_ptr, source_handle, starting_position, slice_length, destination_handle)
    }

    fn mbuffer_eq(&self, m_buffer_handle1: i32, m_buffer_handle2: i32) -> i32 {
        (self.pointers_ptr.mbuffer_eq_func_ptr)(self.context_ptr, m_buffer_handle1, m_buffer_handle2)
    }

    fn mbuffer_set_bytes(&self, m_buffer_handle: i32, data_offset: i32, data_length: i32) -> i32 {
        (self.pointers_ptr.mbuffer_set_bytes_func_ptr)(self.context_ptr, m_buffer_handle, data_offset, data_length)
    }

    fn mbuffer_set_byte_slice(&self, m_buffer_handle: i32, starting_position: i32, data_length: i32, data_offset: i32) -> i32 {
        (self.pointers_ptr.mbuffer_set_byte_slice_func_ptr)(self.context_ptr, m_buffer_handle, starting_position, data_length, data_offset)
    }

    fn mbuffer_append(&self, accumulator_handle: i32, data_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_append_func_ptr)(self.context_ptr, accumulator_handle, data_handle)
    }

    fn mbuffer_append_bytes(&self, accumulator_handle: i32, data_offset: i32, data_length: i32) -> i32 {
        (self.pointers_ptr.mbuffer_append_bytes_func_ptr)(self.context_ptr, accumulator_handle, data_offset, data_length)
    }

    fn mbuffer_to_big_int_unsigned(&self, m_buffer_handle: i32, big_int_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_to_big_int_unsigned_func_ptr)(self.context_ptr, m_buffer_handle, big_int_handle)
    }

    fn mbuffer_to_big_int_signed(&self, m_buffer_handle: i32, big_int_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_to_big_int_signed_func_ptr)(self.context_ptr, m_buffer_handle, big_int_handle)
    }

    fn mbuffer_from_big_int_unsigned(&self, m_buffer_handle: i32, big_int_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_from_big_int_unsigned_func_ptr)(self.context_ptr, m_buffer_handle, big_int_handle)
    }

    fn mbuffer_from_big_int_signed(&self, m_buffer_handle: i32, big_int_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_from_big_int_signed_func_ptr)(self.context_ptr, m_buffer_handle, big_int_handle)
    }

    fn mbuffer_to_big_float(&self, m_buffer_handle: i32, big_float_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_to_big_float_func_ptr)(self.context_ptr, m_buffer_handle, big_float_handle)
    }

    fn mbuffer_from_big_float(&self, m_buffer_handle: i32, big_float_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_from_big_float_func_ptr)(self.context_ptr, m_buffer_handle, big_float_handle)
    }

    fn mbuffer_storage_store(&self, key_handle: i32, source_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_storage_store_func_ptr)(self.context_ptr, key_handle, source_handle)
    }

    fn mbuffer_storage_load(&self, key_handle: i32, destination_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_storage_load_func_ptr)(self.context_ptr, key_handle, destination_handle)
    }

    fn mbuffer_storage_load_from_address(&self, address_handle: i32, key_handle: i32, destination_handle: i32) {
        (self.pointers_ptr.mbuffer_storage_load_from_address_func_ptr)(self.context_ptr, address_handle, key_handle, destination_handle)
    }

    fn mbuffer_get_argument(&self, id: i32, destination_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_get_argument_func_ptr)(self.context_ptr, id, destination_handle)
    }

    fn mbuffer_finish(&self, source_handle: i32) -> i32 {
        (self.pointers_ptr.mbuffer_finish_func_ptr)(self.context_ptr, source_handle)
    }

    fn mbuffer_set_random(&self, destination_handle: i32, length: i32) -> i32 {
        (self.pointers_ptr.mbuffer_set_random_func_ptr)(self.context_ptr, destination_handle, length)
    }

    fn small_int_get_unsigned_argument(&self, id: i32) -> i64 {
        (self.pointers_ptr.small_int_get_unsigned_argument_func_ptr)(self.context_ptr, id)
    }

    fn small_int_get_signed_argument(&self, id: i32) -> i64 {
        (self.pointers_ptr.small_int_get_signed_argument_func_ptr)(self.context_ptr, id)
    }

    fn small_int_finish_unsigned(&self, value: i64) {
        (self.pointers_ptr.small_int_finish_unsigned_func_ptr)(self.context_ptr, value)
    }

    fn small_int_finish_signed(&self, value: i64) {
        (self.pointers_ptr.small_int_finish_signed_func_ptr)(self.context_ptr, value)
    }

    fn small_int_storage_store_unsigned(&self, key_offset: i32, key_length: i32, value: i64) -> i32 {
        (self.pointers_ptr.small_int_storage_store_unsigned_func_ptr)(self.context_ptr, key_offset, key_length, value)
    }

    fn small_int_storage_store_signed(&self, key_offset: i32, key_length: i32, value: i64) -> i32 {
        (self.pointers_ptr.small_int_storage_store_signed_func_ptr)(self.context_ptr, key_offset, key_length, value)
    }

    fn small_int_storage_load_unsigned(&self, key_offset: i32, key_length: i32) -> i64 {
        (self.pointers_ptr.small_int_storage_load_unsigned_func_ptr)(self.context_ptr, key_offset, key_length)
    }

    fn small_int_storage_load_signed(&self, key_offset: i32, key_length: i32) -> i64 {
        (self.pointers_ptr.small_int_storage_load_signed_func_ptr)(self.context_ptr, key_offset, key_length)
    }

    fn int64get_argument(&self, id: i32) -> i64 {
        (self.pointers_ptr.int64get_argument_func_ptr)(self.context_ptr, id)
    }

    fn int64finish(&self, value: i64) {
        (self.pointers_ptr.int64finish_func_ptr)(self.context_ptr, value)
    }

    fn int64storage_store(&self, key_offset: i32, key_length: i32, value: i64) -> i32 {
        (self.pointers_ptr.int64storage_store_func_ptr)(self.context_ptr, key_offset, key_length, value)
    }

    fn int64storage_load(&self, key_offset: i32, key_length: i32) -> i64 {
        (self.pointers_ptr.int64storage_load_func_ptr)(self.context_ptr, key_offset, key_length)
    }

    fn sha256(&self, data_offset: i32, length: i32, result_offset: i32) -> i32 {
        (self.pointers_ptr.sha256_func_ptr)(self.context_ptr, data_offset, length, result_offset)
    }

    fn managed_sha256(&self, input_handle: i32, output_handle: i32) -> i32 {
        (self.pointers_ptr.managed_sha256_func_ptr)(self.context_ptr, input_handle, output_handle)
    }

    fn keccak256(&self, data_offset: i32, length: i32, result_offset: i32) -> i32 {
        (self.pointers_ptr.keccak256_func_ptr)(self.context_ptr, data_offset, length, result_offset)
    }

    fn managed_keccak256(&self, input_handle: i32, output_handle: i32) -> i32 {
        (self.pointers_ptr.managed_keccak256_func_ptr)(self.context_ptr, input_handle, output_handle)
    }

    fn ripemd160(&self, data_offset: i32, length: i32, result_offset: i32) -> i32 {
        (self.pointers_ptr.ripemd160_func_ptr)(self.context_ptr, data_offset, length, result_offset)
    }

    fn managed_ripemd160(&self, input_handle: i32, output_handle: i32) -> i32 {
        (self.pointers_ptr.managed_ripemd160_func_ptr)(self.context_ptr, input_handle, output_handle)
    }

    fn verify_bls(&self, key_offset: i32, message_offset: i32, message_length: i32, sig_offset: i32) -> i32 {
        (self.pointers_ptr.verify_bls_func_ptr)(self.context_ptr, key_offset, message_offset, message_length, sig_offset)
    }

    fn managed_verify_bls(&self, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32 {
        (self.pointers_ptr.managed_verify_bls_func_ptr)(self.context_ptr, key_handle, message_handle, sig_handle)
    }

    fn verify_ed25519(&self, key_offset: i32, message_offset: i32, message_length: i32, sig_offset: i32) -> i32 {
        (self.pointers_ptr.verify_ed25519_func_ptr)(self.context_ptr, key_offset, message_offset, message_length, sig_offset)
    }

    fn managed_verify_ed25519(&self, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32 {
        (self.pointers_ptr.managed_verify_ed25519_func_ptr)(self.context_ptr, key_handle, message_handle, sig_handle)
    }

    fn verify_custom_secp256k1(&self, key_offset: i32, key_length: i32, message_offset: i32, message_length: i32, sig_offset: i32, hash_type: i32) -> i32 {
        (self.pointers_ptr.verify_custom_secp256k1_func_ptr)(self.context_ptr, key_offset, key_length, message_offset, message_length, sig_offset, hash_type)
    }

    fn managed_verify_custom_secp256k1(&self, key_handle: i32, message_handle: i32, sig_handle: i32, hash_type: i32) -> i32 {
        (self.pointers_ptr.managed_verify_custom_secp256k1_func_ptr)(self.context_ptr, key_handle, message_handle, sig_handle, hash_type)
    }

    fn verify_secp256k1(&self, key_offset: i32, key_length: i32, message_offset: i32, message_length: i32, sig_offset: i32) -> i32 {
        (self.pointers_ptr.verify_secp256k1_func_ptr)(self.context_ptr, key_offset, key_length, message_offset, message_length, sig_offset)
    }

    fn managed_verify_secp256k1(&self, key_handle: i32, message_handle: i32, sig_handle: i32) -> i32 {
        (self.pointers_ptr.managed_verify_secp256k1_func_ptr)(self.context_ptr, key_handle, message_handle, sig_handle)
    }

    fn encode_secp256k1_der_signature(&self, r_offset: i32, r_length: i32, s_offset: i32, s_length: i32, sig_offset: i32) -> i32 {
        (self.pointers_ptr.encode_secp256k1_der_signature_func_ptr)(self.context_ptr, r_offset, r_length, s_offset, s_length, sig_offset)
    }

    fn managed_encode_secp256k1_der_signature(&self, r_handle: i32, s_handle: i32, sig_handle: i32) -> i32 {
        (self.pointers_ptr.managed_encode_secp256k1_der_signature_func_ptr)(self.context_ptr, r_handle, s_handle, sig_handle)
    }

    fn add_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, fst_point_xhandle: i32, fst_point_yhandle: i32, snd_point_xhandle: i32, snd_point_yhandle: i32) {
        (self.pointers_ptr.add_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, fst_point_xhandle, fst_point_yhandle, snd_point_xhandle, snd_point_yhandle)
    }

    fn double_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32) {
        (self.pointers_ptr.double_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, point_xhandle, point_yhandle)
    }

    fn is_on_curve_ec(&self, ec_handle: i32, point_xhandle: i32, point_yhandle: i32) -> i32 {
        (self.pointers_ptr.is_on_curve_ec_func_ptr)(self.context_ptr, ec_handle, point_xhandle, point_yhandle)
    }

    fn scalar_base_mult_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: i32, length: i32) -> i32 {
        (self.pointers_ptr.scalar_base_mult_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, data_offset, length)
    }

    fn managed_scalar_base_mult_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32 {
        (self.pointers_ptr.managed_scalar_base_mult_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, data_handle)
    }

    fn scalar_mult_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32, data_offset: i32, length: i32) -> i32 {
        (self.pointers_ptr.scalar_mult_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, point_xhandle, point_yhandle, data_offset, length)
    }

    fn managed_scalar_mult_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, point_xhandle: i32, point_yhandle: i32, data_handle: i32) -> i32 {
        (self.pointers_ptr.managed_scalar_mult_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, point_xhandle, point_yhandle, data_handle)
    }

    fn marshal_ec(&self, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_offset: i32) -> i32 {
        (self.pointers_ptr.marshal_ec_func_ptr)(self.context_ptr, x_pair_handle, y_pair_handle, ec_handle, result_offset)
    }

    fn managed_marshal_ec(&self, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_handle: i32) -> i32 {
        (self.pointers_ptr.managed_marshal_ec_func_ptr)(self.context_ptr, x_pair_handle, y_pair_handle, ec_handle, result_handle)
    }

    fn marshal_compressed_ec(&self, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_offset: i32) -> i32 {
        (self.pointers_ptr.marshal_compressed_ec_func_ptr)(self.context_ptr, x_pair_handle, y_pair_handle, ec_handle, result_offset)
    }

    fn managed_marshal_compressed_ec(&self, x_pair_handle: i32, y_pair_handle: i32, ec_handle: i32, result_handle: i32) -> i32 {
        (self.pointers_ptr.managed_marshal_compressed_ec_func_ptr)(self.context_ptr, x_pair_handle, y_pair_handle, ec_handle, result_handle)
    }

    fn unmarshal_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: i32, length: i32) -> i32 {
        (self.pointers_ptr.unmarshal_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, data_offset, length)
    }

    fn managed_unmarshal_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32 {
        (self.pointers_ptr.managed_unmarshal_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, data_handle)
    }

    fn unmarshal_compressed_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_offset: i32, length: i32) -> i32 {
        (self.pointers_ptr.unmarshal_compressed_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, data_offset, length)
    }

    fn managed_unmarshal_compressed_ec(&self, x_result_handle: i32, y_result_handle: i32, ec_handle: i32, data_handle: i32) -> i32 {
        (self.pointers_ptr.managed_unmarshal_compressed_ec_func_ptr)(self.context_ptr, x_result_handle, y_result_handle, ec_handle, data_handle)
    }

    fn generate_key_ec(&self, x_pub_key_handle: i32, y_pub_key_handle: i32, ec_handle: i32, result_offset: i32) -> i32 {
        (self.pointers_ptr.generate_key_ec_func_ptr)(self.context_ptr, x_pub_key_handle, y_pub_key_handle, ec_handle, result_offset)
    }

    fn managed_generate_key_ec(&self, x_pub_key_handle: i32, y_pub_key_handle: i32, ec_handle: i32, result_handle: i32) -> i32 {
        (self.pointers_ptr.managed_generate_key_ec_func_ptr)(self.context_ptr, x_pub_key_handle, y_pub_key_handle, ec_handle, result_handle)
    }

    fn create_ec(&self, data_offset: i32, data_length: i32) -> i32 {
        (self.pointers_ptr.create_ec_func_ptr)(self.context_ptr, data_offset, data_length)
    }

    fn managed_create_ec(&self, data_handle: i32) -> i32 {
        (self.pointers_ptr.managed_create_ec_func_ptr)(self.context_ptr, data_handle)
    }

    fn get_curve_length_ec(&self, ec_handle: i32) -> i32 {
        (self.pointers_ptr.get_curve_length_ec_func_ptr)(self.context_ptr, ec_handle)
    }

    fn get_priv_key_byte_length_ec(&self, ec_handle: i32) -> i32 {
        (self.pointers_ptr.get_priv_key_byte_length_ec_func_ptr)(self.context_ptr, ec_handle)
    }

    fn elliptic_curve_get_values(&self, ec_handle: i32, field_order_handle: i32, base_point_order_handle: i32, eq_constant_handle: i32, x_base_point_handle: i32, y_base_point_handle: i32) -> i32 {
        (self.pointers_ptr.elliptic_curve_get_values_func_ptr)(self.context_ptr, ec_handle, field_order_handle, base_point_order_handle, eq_constant_handle, x_base_point_handle, y_base_point_handle)
    }
}