//! # Utilities
//!
//! A collection of small utilities that extend the Rust standard library with
//! features required by this crate.

/// Return the absolute directory of a file path
///
/// This takes a path to a file and returns the absolute path to the directory
/// holding the file. This will query `std::env::current_dir()` if the
/// specified path is relative.
///
/// This function always succeeds and is never ambiguous. However, if the
/// specified path points to something other than a file, ambiguous results
/// might be returned. Hence, this is not supported.
///
/// This function only ever operates on the path. It never queries the file
/// system nor requires the path to exist on the system.
pub fn absdir(path: &dyn AsRef<std::path::Path>) -> std::path::PathBuf {
    // Use CWD as base in case the path is relative.
    let mut b = std::env::current_dir().expect("cannot query current working directory");

    // Push on top of the CWD. For absolute paths this will truncate first.
    b.push(path.as_ref());

    // We want the parent directory of the file, so strip the final component.
    // Since the path is absolute at this point, this cannot fail, and is never
    // ambiguous. So `pop()` cannot fail and will never yield an empty path.
    assert_eq!(b.pop(), true);

    b
}

/// ## Escape XML PCDATA
///
/// Create a new string that has the same content as the input but all special
/// characters encoded suitably for XML PCDATA.
pub fn escape_xml_pcdata(input: &str) -> String {
    let n_alloc = input.chars().fold(
        0,
        |acc, v| match v {
            '&' => acc + 5,
            '<' => acc + 4,
            _ => acc + 1,
        }
    );

    let mut v = String::with_capacity(n_alloc);

    for c in input.chars() {
        match c {
            '&' => v.push_str("&amp;"),
            '<' => v.push_str("&lt;"),
            _ => v.push(c),
        }
    }

    v
}

/// ## Turn strings into valid symbol identifiers
///
/// Create a new string that has the same content as the input but all
/// unsupported characters replaced by an underscore. Only alphanumeric
/// characters are supported (but the full unicode range).
///
/// Additionally, if the string starts with a numeric character, it is
/// prefixed with an underscore.
pub fn symbolize(input: &str) -> String {
    let needs_prefix = input.chars()
        .next()
        .map(char::is_numeric)
        .unwrap_or(true);

    let mut v = String::with_capacity(input.len() + (needs_prefix as usize));

    if needs_prefix {
        v.push('_');
    }

    for c in input.chars() {
        if c.is_alphanumeric() {
            v.push(c);
        } else {
            v.push('_');
        }
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    // Run tests against `absdir()` and verify that it will properly
    // return the parent directory as an absolute path.
    #[test]
    fn test_absdir() {
        let mut cwd = std::env::current_dir().unwrap();

        assert_eq!(
            absdir(&"/foo"),
            std::path::Path::new("/"),
        );

        assert_eq!(
            absdir(&"/foo/bar").as_path(),
            std::path::Path::new("/foo"),
        );

        assert_eq!(
            absdir(&"/foo/../bar"),
            std::path::Path::new("/foo/.."),
        );

        assert_eq!(
            absdir(&"foo").as_path(),
            cwd.as_path(),
        );

        cwd.push("foo");
        assert_eq!(
            absdir(&"foo/bar").as_path(),
            cwd.as_path(),
        );
        cwd.pop();

        cwd.push("..");
        assert_eq!(
            absdir(&"../bar").as_path(),
            cwd.as_path(),
        );
        cwd.pop();
    }

    // Verify that the XML-PCDATA escapes are properly handled.
    #[test]
    fn test_xml_pcdata() {
        assert_eq!(escape_xml_pcdata(""), "");
        assert_eq!(escape_xml_pcdata("foobar"), "foobar");
        assert_eq!(escape_xml_pcdata("foo & bar"), "foo &amp; bar");
        assert_eq!(escape_xml_pcdata("<foobar>"), "&lt;foobar>");
        assert_eq!(escape_xml_pcdata("<&>"), "&lt;&amp;>");
    }

    // Run some basic string conversion tests on the `symbolize()` helper. It
    // should properly preprend prefixes and replace unsupported characters.
    #[test]
    fn symbolize_basic() {
        assert_eq!(symbolize(""), "_");
        assert_eq!(symbolize("foobar"), "foobar");
        assert_eq!(symbolize("0foobar"), "_0foobar");
        assert_eq!(symbolize("foo-bar"), "foo_bar");
        assert_eq!(symbolize("0foo-bar"), "_0foo_bar");
        assert_eq!(symbolize("foo(bar)"), "foo_bar_");
    }
}
