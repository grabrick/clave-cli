#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Overlay {
    #[default]
    None,
    Effort,
    Settings,
    Chats,
}

impl Overlay {
    pub(crate) fn is_open(self) -> bool {
        self != Overlay::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_not_open_others_are() {
        assert!(!Overlay::None.is_open());
        assert!(Overlay::Effort.is_open());
        assert!(Overlay::Settings.is_open());
        assert_eq!(Overlay::default(), Overlay::None);
    }
}
