use super::*;

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn should_number_a_repeated_device_name_from_its_second_device_on() {
    // When
    let unique = numbered(&names(&["USB Mic", "MacBook Microphone", "USB Mic"]));
    // Then
    assert_eq!(
        unique,
        names(&["USB Mic", "MacBook Microphone", "USB Mic #2"])
    );
}

#[test]
fn should_find_the_device_the_configuration_names_exactly() {
    // When
    let found = position(&names(&["USB Mic", "USB Mic #2"]), "USB Mic #2");
    // Then
    assert_eq!(found, Some(1));
}

#[test]
fn should_pick_no_device_for_the_system_default() {
    // When
    let found = position(&names(&["default", "USB Mic"]), DEFAULT_DEVICE);
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_pick_no_device_when_the_named_one_is_not_connected() {
    // When
    let found = position(&names(&["USB Mic"]), "AirPods");
    // Then
    assert_eq!(found, None);
}
