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
        ("эйст", "paste"),
        ("ейст", "paste"),
        ("хейст", "paste"),
    ];
    for (cyrillic, english) in heard {
        // When
        let respelled = respell_word(cyrillic);
        // Then
        assert_eq!(respelled, Some(vec![english]), "{cyrillic}");
    }
}

#[test]
fn should_keep_the_russian_word_for_eats() {
    // When "ест" is heard: "paste" without its "п" is respelled only with its "й"
    let respelled = respell_word("ест");
    // Then
    assert_eq!(respelled, None);
}

#[test]
fn should_split_a_glued_double_into_two_words() {
    // When
    let respelled = respell_word("энтерэнтер");
    // Then
    assert_eq!(respelled, Some(vec!["enter", "enter"]));
}

#[test]
fn should_respell_the_users_mishearings_from_the_command_history() {
    // Given the words the Russian recognizer wrote for the user's commands in the history
    let heard = [
        ("лентер", vec!["enter"]),
        ("яндер", vec!["enter"]),
        ("поист", vec!["paste"]),
        ("клио", vec!["clear"]),
        ("клет", vec!["clear"]),
        ("тартарт", vec!["start", "start"]),
        ("стартарт", vec!["start", "start"]),
        ("тарстард", vec!["start", "start"]),
        ("рестард", vec!["restart"]),
    ];
    for (cyrillic, english) in heard {
        // When
        let respelled = respell_word(cyrillic);
        // Then
        assert_eq!(respelled, Some(english), "{cyrillic}");
    }
}

#[test]
fn should_leave_russian_words_close_to_a_command() {
    // When
    let respelled = respell("пост центр клевер стартовый");
    // Then
    assert_eq!(respelled, "пост центр клевер стартовый");
}

#[test]
fn should_respell_only_the_command_words_of_a_long_form() {
    // When the long forms, no commands of their own, are written in Cyrillic
    let respelled = respell("пресс энтер, старт диктейшн, клир ол текст");
    // Then only their command words are respelled
    assert_eq!(respelled, "пресс enter, start диктейшн, clear ол текст");
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

#[test]
fn should_read_each_glued_or_mangled_enter_from_the_history() {
    // Given the words the Russian recognizer wrote for "enter" and "enter enter" in the
    // user's command history: glued doubles, a glued triple and mangled singles
    let doubles = [
        "интернтер",
        "интерентер",
        "энтерентор",
        "энтеррентор",
        "интернетор",
        "интеррентор",
        "интерендер",
        "интеррендер",
        "энтернтер",
        "интеринтер",
        "энтер-энтер",
        "энтеренторендрента",
    ];
    let singles = [
        "нтр",
        "энтро",
        "рентор",
        "энтор",
        "энтар",
        "нтэр",
        "тентер",
        "рентра",
    ];
    for word in doubles {
        // When
        let respelled = respell_word(word);
        // Then
        assert_eq!(respelled, Some(vec!["enter", "enter"]), "{word}");
    }
    for word in singles {
        assert_eq!(respell_word(word), Some(vec!["enter"]), "{word}");
    }
}

#[test]
fn should_leave_ordinary_words_with_the_letters_of_enter() {
    // Given ordinary Russian words that hold the letters of "enter"
    let text = "интересно интересная интерфейса интервью центр контракт контроллер линтры интернет";
    // When
    let respelled = respell(text);
    // Then
    assert_eq!(respelled, text);
}
