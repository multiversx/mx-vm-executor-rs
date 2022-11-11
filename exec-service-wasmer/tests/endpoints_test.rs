mod common;

#[test]
fn instance_endpoints_empty() {
    let instance = common::test_instance(common::EMPTY_SC_WAT);
    assert_eq!(
        instance.get_exported_function_names(),
        vec!["init", "callBack"]
    );
}

#[test]
fn instance_endpoints_adder() {
    let instance = common::test_instance(common::ADDER_WAT);
    assert!(instance.has_function("add"));
    assert!(!instance.has_function("missingEndpoint"));
    assert_eq!(
        instance.get_exported_function_names(),
        vec!["init", "add", "getSum", "callBack"]
    );
}
