#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttSignal {
    Pressed,
    Released,
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_compare_pressed_equal_to_pressed() {
        // Given
        let a = PttSignal::Pressed;
        let b = PttSignal::Pressed;
        // When
        let result = a == b;
        // Then
        assert!(result);
    }

    #[test]
    fn should_compare_pressed_not_equal_to_released() {
        // Given
        let a = PttSignal::Pressed;
        let b = PttSignal::Released;
        // When
        let result = a == b;
        // Then
        assert!(!result);
    }
}
