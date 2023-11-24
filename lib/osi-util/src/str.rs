//! # String Utilities
//!
//! This module provides utilities for string operations, which are not found
//! in the standard library.

/// ## Compare Strings with Natural Sort Order
///
/// This takes two strings and compares them with natural sort order, trying
/// to interpret digit runs as natural numbers.
pub fn cmp_natural(
    mut lhs: &str,
    mut rhs: &str,
) -> core::cmp::Ordering {
    // Advance over a string by splitting off a non-digit prefix, followed by
    // a digit-only prefix. The prefixes are yielded to the caller.
    fn advance<'a>(
        stream: &mut &'a str,
    ) -> (&'a str, &'a str) {
        let rem = *stream;

        // Split off non-digit prefix.
        let (name, rem) = rem.split_at(
            rem.find(|v: char| v.is_numeric())
                .unwrap_or(rem.len()),
        );

        // Split off digit-only prefix.
        let (number, rem) = rem.split_at(
            rem.find(|v: char| !v.is_numeric())
                .unwrap_or(rem.len()),
        );

        // Advance stream and return the name+number tuple.
        *stream = rem;
        (name, number)
    }

    // Advance both sides one by one and compare each token individually.
    while !lhs.is_empty() || !rhs.is_empty() {
        let (l_name, l_num) = advance(&mut lhs);
        let (r_name, r_num) = advance(&mut rhs);
        let l_u64 = l_num.parse::<u64>();
        let r_u64 = r_num.parse::<u64>();

        // Compare the non-digit prefix.
        match l_name.cmp(r_name) {
            v @ core::cmp::Ordering::Less => return v,
            v @ core::cmp::Ordering::Greater => return v,
            _ => {},
        }

        // Compare the digit-only prefix as u64, if possible. Note that
        // different strings can map to the same u64, so even if both u64s
        // are equal, we have to continue comparing their original string
        // representation.
        match (l_u64, r_u64) {
            (Ok(l), Ok(r)) => match l.cmp(&r) {
                v @ core::cmp::Ordering::Less => return v,
                v @ core::cmp::Ordering::Greater => return v,
                _ => {},
            },
            _ => {},
        }

        // Compare the digit-only prefix as string.
        match l_num.cmp(r_num) {
            v @ core::cmp::Ordering::Less => return v,
            v @ core::cmp::Ordering::Greater => return v,
            _ => {},
        }
    }

    core::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify Natural Sort Order
    //
    // Check that `cmp_natural()` orders based on the natural sort order,
    // rather than on lexicographic sort order.
    #[test]
    fn cmp_natural_basic() {
        assert_eq!(
            cmp_natural("foobar", "foobar"),
            core::cmp::Ordering::Equal,
        );
        assert_eq!(
            cmp_natural("foobar0", "foobar1"),
            core::cmp::Ordering::Less,
        );
        assert_eq!(
            cmp_natural("foobar1", "foobar0"),
            core::cmp::Ordering::Greater,
        );
        assert_eq!(
            cmp_natural("foobar2", "foobar10"),
            core::cmp::Ordering::Less,
        );
        assert_eq!(
            cmp_natural("foo2bar3", "foo10bar10"),
            core::cmp::Ordering::Less,
        );
    }
}
