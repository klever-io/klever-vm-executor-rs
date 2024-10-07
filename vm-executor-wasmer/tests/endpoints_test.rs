use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use klever_chain_vm_executor::{ExecutorService, VMHooksDefault};
use klever_chain_vm_executor_wasmer::BasicExecutorService;
use crate::common::DUMMY_COMPILATION_OPTIONS;

mod common;

#[test]
fn instance_endpoints_empty() {
    let instance = common::test_instance(common::EMPTY_SC_WAT).unwrap();
    assert_eq!(
        instance.get_exported_function_names(),
        vec!["init", "callBack"]
    );
}

#[test]
fn instance_endpoints_adder() {
    let instance = common::test_instance(common::ADDER_WAT).unwrap();
    assert!(instance.has_function("add"));
    assert!(!instance.has_function("missingEndpoint"));
    assert_eq!(
        instance.get_exported_function_names(),
        vec!["init", "add", "getSum", "callBack"]
    );
}

#[test]
fn capture_module_compilation_panics() {
    let result = common::test_instance(common::UNSUPPORTED_OPERATIONS);
    assert!(result.is_err());
    let error = result.err().unwrap();
    assert_eq!(error.to_string(), "module compilation panicked");
}

#[test]
fn bad_init_param() {
    let instance = common::test_instance(common::BAD_INIT_PARAM).unwrap();
    assert!(!instance.check_signatures());
}

#[test]
fn bad_init_result() {
    let instance = common::test_instance(common::BAD_INIT_RESULT).unwrap();
    assert!(!instance.check_signatures());
}

#[test]
fn test_functions_constraints() {
    // code bloat attack, should fail
    let bytes = get_file_as_byte_vec("tests/assets/code_bloat_attack.wasm");
    let service = BasicExecutorService::new();
    let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    executor.new_instance(&bytes, &DUMMY_COMPILATION_OPTIONS).err().expect("should fail");

    // web3 dns, should pass
    let bytes = get_file_as_byte_vec("tests/assets/web3-dns.wasm");
    let service = BasicExecutorService::new();
    let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    executor.new_instance(&bytes, &DUMMY_COMPILATION_OPTIONS).expect("should pass");
}

fn get_file_as_byte_vec(contract_name: &str) -> Vec<u8> {
    // relative path from cargo
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push(contract_name);
    let filename = d.to_str().unwrap().to_string();

    // load contents
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");

    buffer
}
