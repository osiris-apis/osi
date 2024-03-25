//! # Miscellaneous Utilities
//!
//! A collection of small utilities that extend the Rust standard library with
//! features required by this crate.

/// ## Return the absolute directory of a file path
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
    assert!(b.pop());

    b
}

/// Escape suitably for single-quote usage. This will prefix any single quote
/// or backslash ASCII character with a backslash.
pub fn escape_single_quote(input: &std::ffi::OsStr) -> std::ffi::OsString {
    // SAFETY: The encoded-bytes representation is self-synchronizing and a
    //         superset of UTF-8 and thus ASCII. Hence, by just looking at the
    //         ASCII characters, we can safely reassemble the stream.
    let from = input.as_encoded_bytes();
    let n_alloc = from.iter().fold(
        0,
        |acc, v| match v {
            b'\'' | b'\\' => acc + 2,
            _ => acc + 1,
        },
    );

    let mut to = Vec::with_capacity(n_alloc);

    for &c in from.iter() {
        if c == b'\'' || c == b'\\' {
            to.push(b'\\');
        }
        to.push(c);
    }

    // SAFETY: See above.
    unsafe { std::ffi::OsString::from_encoded_bytes_unchecked(to) }
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

/// Reduce a line of text to a fixed width, highlighting a selected range. Use
/// it to reduce overlong lines when displaying on a limited device.
///
/// This will split the line into 2 sections, possibly stripping text before,
/// between, and after the sections. The sections will be positioned to show
/// the start and end of the highlighted region, plus leading and trailing
/// context. If possible, the highlighted region is preserved in whole.
pub fn ellipse(
    line: &str,
    mut range: core::ops::Range<usize>,
    width: usize,
) -> (core::ops::Range<usize>, core::ops::Range<usize>) {
    let n_line = line.len();

    // Normalize the range to avoid overflows in arithmetic.
    if range.start > n_line {
        range.start = n_line;
    }
    if range.end > n_line {
        range.end = n_line;
    }
    if range.start > range.end {
        range.start = range.end;
    }
    let n_range = range.end - range.start;

    // Divide the window into zones, where the first and last quarters are
    // reserved for leading and trailing context, and the center half is used
    // for the highlighted region.
    // The divide is arbitrary, but seems to work well. If needed, this can be
    // turned into a function argument.
    let zone = width / 4;

    // Set zone-based borders left and right, which mark where leading and
    // trailing context is capped.
    let mut border_left = range.start.saturating_sub(zone);
    let mut border_right = core::cmp::min(range.end.saturating_add(zone), n_line);

    // Increase leading context if we have enough space.
    let rem = width.saturating_sub(border_right.saturating_sub(border_left));
    let n = core::cmp::min(border_left, rem);
    border_left -= n;

    // Increase trailing context if we still have enough space.
    let rem = rem - n;
    let n = core::cmp::min(n_line.saturating_sub(border_right), rem);
    border_right += n;

    // If we do not have enough space, create a cut in the center, using some
    // approximation of a golden-ratio.
    let n = border_right.saturating_sub(border_left).saturating_sub(width);
    let cut_left = range.start + (n_range.saturating_mul(618) / 1000);
    let cut_right = cut_left + n;

    (
        core::ops::Range {
            start: border_left,
            end: cut_left,
        },
        core::ops::Range {
            start: cut_right,
            end: border_right,
        },
    )
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

    // Verify that the single-quote escapes are properly handled.
    #[test]
    fn test_single_quote() {
        assert_eq!(
            escape_single_quote(std::ffi::OsStr::new("")),
            "",
        );
        assert_eq!(
            escape_single_quote(std::ffi::OsStr::new("foobar")),
            "foobar",
        );
        assert_eq!(
            escape_single_quote(std::ffi::OsStr::new("foo'bar")),
            "foo\\'bar",
        );
        assert_eq!(
            escape_single_quote(std::ffi::OsStr::new("'foo\\'bar'")),
            "\\'foo\\\\\\'bar\\'",
        );
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
}
