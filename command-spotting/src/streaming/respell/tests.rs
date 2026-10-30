use super::*;

#[test]
fn should_respell_each_command_word_heard_in_cyrillic() {
    // Given the user's command words as the Russian recognizer writes them
    let heard = [
        ("стоп", "stop"),
        ("стап", "stop"),
        ("старт", "start"),
        ("стат", "start"),
        ("рестарт", "restart"),
        ("ристарт", "restart"),
        ("энтер", "enter"),
        ("интер", "enter"),
        ("ентор", "enter"),
        ("клир", "clear"),
        ("клиар", "clear"),
        ("кэнсел", "cancel"),
        ("кансл", "cancel"),
        ("пейст", "paste"),
        ("пэст", "paste"),
    ];
    for (cyrillic, english) in heard {
        // When
        let respelled = respell_word(cyrillic);
        // Then
        assert_eq!(respelled, Some(vec![english]), "{cyrillic}");
    }
}

#[test]
fn should_split_a_glued_double_into_two_words() {
    // When
    let respelled = respell_word("энтерэнтер");
    // Then
    assert_eq!(respelled, Some(vec!["enter", "enter"]));
}

#[test]
fn should_respell_the_other_words_of_a_command_form() {
    // When
    let respelled = respell("пресс энтер плиз, старт диктейшн, клир ол текст");
    // Then
    assert_eq!(
        respelled,
        "press enter please, start dictation, clear all text"
    );
}

#[test]
fn should_leave_a_word_that_is_not_only_command_sounds() {
    // When
    let respelled = respell("привет стопка стоп");
    // Then
    assert_eq!(respelled, "привет стопка stop");
}

#[test]
fn should_read_yo_as_ye_and_ignore_case() {
    // When
    let respelled = respell("Энтёр");
    // Then
    assert_eq!(respelled, "enter");
}

#[test]
fn should_leave_latin_text_untouched() {
    // When
    let respelled = respell("Press enter.");
    // Then
    assert_eq!(respelled, "Press enter.");
}
