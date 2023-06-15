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

#[test]
fn bad_init_param() {
    let instance = common::test_instance(common::BAD_INIT_PARAM);
    assert!(!instance.check_signatures());
}

#[test]
fn bad_init_result() {
    let instance = common::test_instance(common::BAD_INIT_RESULT);
    assert!(!instance.check_signatures());
}
