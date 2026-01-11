/// Helper to assert a displayable type contains specific substring snippets and/or lines
pub trait AssertContains: ToString {
    /// Asserts that the [`ToString`] representation contains all the specified text snippets
    ///
    /// Returns the string for convenient chanining
    ///
    /// # Examples
    ///
    /// ```
    /// # use kopia_exporter::AssertContains;
    /// "hello this has words".assert_contains_snippets(&[
    ///     "this",
    ///     "as",
    ///     "hello this",
    /// ]);
    /// ```
    ///
    /// ```should_panic
    /// # use kopia_exporter::AssertContains;
    /// "hello this has words".assert_contains_snippets(&[
    ///     "nonexistent", // panic: missing from input string
    /// ]);
    /// ```
    ///
    /// ```should_panic
    /// # use kopia_exporter::AssertContains;
    /// "hello this has words".assert_contains_snippets(&[]); // panic: empty list
    /// ```
    #[track_caller]
    fn assert_contains_snippets(&self, snippets: &[&str]) -> String {
        assert!(
            !snippets.is_empty(),
            "refusing empty list for assert_contains_all"
        );

        let s = self.to_string();
        for snippet in snippets {
            assert!(
                s.contains(snippet),
                r#"expected:
"""
{snippet}
"""
to be contained in found string:
"""
{s}
""""#
            );
        }
        s
    }

    /// Asserts that the [`ToString`] representation contains all the specified lines
    ///
    /// Returns the string for convenient chanining
    ///
    /// # Examples
    ///
    /// ```
    /// # use kopia_exporter::AssertContains;
    /// "string with
    /// lots of text across
    /// multiple lines".assert_contains_lines(&[
    ///     "multiple lines",
    ///     "lots of text across",
    /// ]);
    /// ```
    ///
    /// ```should_panic
    /// # use kopia_exporter::AssertContains;
    /// "string with
    /// lots of text across
    /// multuple lines".assert_contains_lines(&[
    ///     "multiple lines",
    ///     "lots of text ", // <-- panic: missing tailing word "across"
    /// ]);
    /// ```
    ///
    /// ```should_panic
    /// # use kopia_exporter::AssertContains;
    /// "hello this has words".assert_contains_lines(&[]); // panic: empty list
    /// ```
    #[track_caller]
    fn assert_contains_lines(&self, lines: &[&str]) -> String {
        assert!(
            !lines.is_empty(),
            "refusing empty list for assert_contains_all"
        );

        let s = self.to_string();
        for line in lines {
            assert!(
                s.lines().any(|l| l == *line),
                r#"expected line:
"""
{line}
"""
to be contained in found string:
"""
{s}
""""#
            );
        }
        s
    }
}
impl<T> AssertContains for T where T: ToString {}
