use crate::Source;

impl Source {
    /// Converts from the JSON/typed [`Source`] to a flat string [`SourceStr`]
    ///
    /// # Errors
    /// Returns an error if the `user_name` or `host` contain invalid characters that would
    /// make the flat string representation ambiguous
    pub fn render(&self) -> Result<SourceStr, Error> {
        let Self {
            host,
            user_name,
            path,
        } = self;

        let make_err = |kind| {
            Err(Error {
                kind,
                value_source: self.clone(),
            })
        };

        // reject invalid characters, to perserve uniqueness for SourceStr representation
        {
            let invalid_char = '@';
            if user_name.contains(invalid_char) {
                return make_err(ErrorKind::InvalidUserName {
                    user_name: user_name.clone(),
                    invalid_char,
                });
            }
        }
        {
            let invalid_char = ':';
            if host.contains(invalid_char) {
                return make_err(ErrorKind::InvalidHost {
                    host: host.clone(),
                    invalid_char,
                });
            }
        }

        let rendered = format!("{user_name}@{host}:{path}");
        Ok(SourceStr(rendered))
    }
}
/// String version for a [`Source`] rendered for output
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceStr(String);
impl SourceStr {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Debug for SourceStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(text) = self;
        // wrap in Debug, to escape quotes
        write!(f, "{text:?}")
    }
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    value_source: Source,
}
#[derive(Debug)]
enum ErrorKind {
    InvalidUserName {
        user_name: String,
        invalid_char: char,
    },
    InvalidHost {
        host: String,
        invalid_char: char,
    },
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.kind {
            ErrorKind::InvalidUserName { .. } | ErrorKind::InvalidHost { .. } => None,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { kind, value_source } = self;
        match kind {
            ErrorKind::InvalidUserName {
                user_name,
                invalid_char,
            } => {
                write!(
                    f,
                    "invalid char {invalid_char:?} in user name {user_name:?}"
                )
            }
            ErrorKind::InvalidHost { host, invalid_char } => {
                write!(f, "invalid char {invalid_char:?} in host {host:?}")
            }
        }?;
        write!(f, " in {value_source:?}")
    }
}
