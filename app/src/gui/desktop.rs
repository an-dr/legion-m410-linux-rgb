/// The desktop whose look the interface borrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Desktop {
    Cosmic,
    Gnome,
    Kde,
}

impl Desktop {
    /// From `XDG_CURRENT_DESKTOP`, which lists desktops separated by colons.
    /// Anything unrecognized borrows GNOME's look, which suits most others.
    pub fn detect() -> Self {
        Self::from_names(&std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default())
    }

    pub fn from_names(names: &str) -> Self {
        let lower = names.to_ascii_lowercase();
        let has = |name: &str| lower.split(':').any(|part| part == name);
        if has("cosmic") {
            Desktop::Cosmic
        } else if has("kde") {
            Desktop::Kde
        } else {
            Desktop::Gnome
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_is_read_from_the_colon_separated_list() {
        assert_eq!(Desktop::from_names("COSMIC"), Desktop::Cosmic);
        assert_eq!(Desktop::from_names("pop:COSMIC"), Desktop::Cosmic);
        assert_eq!(Desktop::from_names("KDE"), Desktop::Kde);
        assert_eq!(Desktop::from_names("ubuntu:GNOME"), Desktop::Gnome);
    }

    #[test]
    fn an_unknown_desktop_gets_a_look_rather_than_none() {
        assert_eq!(Desktop::from_names(""), Desktop::Gnome);
        assert_eq!(Desktop::from_names("XFCE"), Desktop::Gnome);
    }
}
