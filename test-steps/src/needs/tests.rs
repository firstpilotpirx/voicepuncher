use super::required_env;

const SET: &str = "VOICEPUNCHER_TEST_STEPS_SET";
const UNSET: &str = "VOICEPUNCHER_TEST_STEPS_UNSET";

#[test]
fn should_return_the_value_of_a_set_variable() {
    // Given
    std::env::set_var(SET, "http://127.0.0.1:8081");
    // When
    let value = required_env(SET);
    // Then
    assert_eq!(value, "http://127.0.0.1:8081");
}

#[test]
fn should_fail_naming_the_variable_and_the_gates_when_it_is_unset() {
    // Given
    std::env::remove_var(UNSET);
    // When
    let failure = std::panic::catch_unwind(|| required_env(UNSET)).unwrap_err();
    // Then
    let message = failure.downcast_ref::<String>().unwrap();
    assert!(message.contains(UNSET), "{message}");
    assert!(message.contains("the gates of `dev-tools`"), "{message}");
}

#[test]
fn should_name_the_recipe_that_provides_the_missing_models() {
    // Given
    std::env::remove_var("VOICEPUNCHER_KWS_MODEL_DIR");
    // When
    let failure =
        std::panic::catch_unwind(|| required_env("VOICEPUNCHER_KWS_MODEL_DIR")).unwrap_err();
    // Then
    let message = failure.downcast_ref::<String>().unwrap();
    assert!(message.contains("just models"), "{message}");
}
