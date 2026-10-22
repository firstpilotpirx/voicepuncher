use super::*;

#[test]
fn should_place_each_menu_entry_where_the_menu_lists_it() {
    // Given / When
    let places: Vec<isize> = [
        MenuEntry::TitleLine,
        MenuEntry::StatusLine,
        MenuEntry::DownloadModels,
        MenuEntry::Services,
    ]
    .map(MenuEntry::place)
    .to_vec();
    // Then
    assert_eq!(places, vec![0, 1, 3, 4]);
    for (index, entry) in MENU.iter().enumerate() {
        if *entry != MenuEntry::Separator {
            assert_eq!(entry.place(), isize::try_from(index).unwrap());
        }
    }
}
