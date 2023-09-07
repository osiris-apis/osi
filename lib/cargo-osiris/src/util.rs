//! # Utilities
//!
//! A collection of small utilities that extend the Rust standard library with
//! features required by this crate.

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
