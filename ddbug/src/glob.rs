//! A simple glob matcher for paths.

/// Return true if the glob pattern matches a path.
///
/// - `?` matches any single character except `/`
/// - `*` matches any sequence of characters except `/`
/// - `/**/` matches zero or more complete path components
/// - all other characters match themselves
///
/// The pattern must match complete components of the path,
/// but it may start matching at any path component (implicit leading `**/`),
/// and optionally finish matching at a trailing `/` (implicit trailing `**`).
pub(crate) fn matches(pattern: &str, path: &str) -> bool {
    let path = path.as_bytes();
    // Check if pattern finishes with a '/'.
    let (pattern, directory) = match pattern.as_bytes() {
        // Also ignore redundant trailing `**`.
        [p @ .., b'/', b'*', b'*', b'/'] | [p @ .., b'/', b'*', b'*'] | [p @ .., b'/'] => (p, true),
        p => (p, false),
    };
    // Ignore redundant leading `**/`.
    let pattern = pattern.strip_prefix(b"**/").unwrap_or(pattern);
    if match_all(pattern, path, directory) {
        return true;
    }
    // Try matching at the start of each path component.
    (1..=path.len())
        .filter(|&offset| path[offset - 1] == b'/')
        .any(|offset| match_all(pattern, &path[offset..], directory))
}

/// Return true if the glob pattern matches all of the path.
///
/// If `directory` is true then the pattern instead matches a directory prefix
/// of the path.
fn match_all(mut pattern: &[u8], mut haystack: &[u8], directory: bool) -> bool {
    struct Backtrack<'a, 'b> {
        pattern: &'a [u8],
        haystack: &'b [u8],
    }

    // The most recent '*' pattern.
    let mut star = None;
    // The most recent '**/' pattern.
    let mut star_slash = None;
    // Whether '*' is more recent than than '**/'.
    let mut star_inner = false;
    loop {
        match pattern {
            [b'/', b'*', b'*', b'/', p @ ..] => {
                if let [c, h @ ..] = haystack
                    && *c == b'/'
                {
                    pattern = p;
                    haystack = h;
                    star_slash = Some(Backtrack { pattern, haystack });
                    star_inner = false;
                    continue;
                }
            }
            [b'*', p @ ..] => {
                pattern = p;
                star = Some(Backtrack { pattern, haystack });
                star_inner = true;
                continue;
            }
            [b'?', p @ ..] => {
                if let Some(h) = skip_char(haystack) {
                    pattern = p;
                    haystack = h;
                    continue;
                }
            }
            [c1, p @ ..] => {
                if let [c2, h @ ..] = haystack
                    && c1 == c2
                {
                    pattern = p;
                    haystack = h;
                    continue;
                }
            }
            [] => {
                if directory {
                    // Match everything below the directory.
                    if haystack.first() == Some(&b'/') {
                        return true;
                    }
                } else {
                    if haystack.is_empty() {
                        return true;
                    }
                }
            }
        }

        // Backtrack by extending the most recent wildcard.
        loop {
            if star_inner {
                let Some(bt) = &mut star else {
                    return false;
                };

                // Extend the `*` by one character, but not past a `/`.
                if let Some(h) = skip_char(bt.haystack) {
                    bt.haystack = h;
                    pattern = bt.pattern;
                    haystack = h;
                    break;
                }

                star = None;
                star_inner = false;
            } else {
                let Some(bt) = &mut star_slash else {
                    return false;
                };

                // Extend the `/**/` by one complete path component.
                if let Some(index) = bt.haystack.iter().position(|&c| c == b'/') {
                    bt.haystack = &bt.haystack[index + 1..];
                    pattern = bt.pattern;
                    haystack = bt.haystack;
                    break;
                }

                star_slash = None;
                star_inner = true;
            }
        }
    }
}

/// Skip one character from the start of the path, and return the remainder.
///
/// Returns `None` if the path is empty or starts with a `/` or invalid character.
///
/// Note: does not fully validate the UTF-8.
fn skip_char(haystack: &[u8]) -> Option<&[u8]> {
    match haystack {
        [] | [b'/', ..] => None,
        [c, h @ ..] => match c {
            0x00..=0x7f => Some(h),
            0xc0..=0xdf => h.get(1..),
            0xe0..=0xef => h.get(2..),
            0xf0..=0xf7 => h.get(3..),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn literal() {
        assert!(matches("src/file.c", "src/file.c"));
        assert!(!matches("src/file.c", "src/myfile.c"));
        assert!(!matches("src/file.c", "src/fill.c"));
    }

    #[test]
    fn empty() {
        assert!(matches("", ""));
        assert!(!matches("", "src/file.c"));
    }

    #[test]
    fn end() {
        assert!(matches("src/file.c", "src/file.c"));
        assert!(!matches("src/file.c", "src/file"));
        assert!(!matches("src/file", "src/file.c"));
    }

    #[test]
    fn start() {
        assert!(matches("file.c", "src/file.c"));
        assert!(!matches("file.c", "src/myfile.c"));
        assert!(matches("src/file.c", "/home/user/src/file.c"));
        assert!(!matches("src/file.c", "/home/user/mysrc/file.c"));
    }

    #[test]
    fn root() {
        assert!(matches("/home/user/src/file.c", "/home/user/src/file.c"));
        assert!(!matches("/src/file.c", "/home/user/src/file.c"));
        assert!(matches("/home/", "/home/user/src/file.c"));
        assert!(!matches("/src/", "/home/user/src/file.c"));
    }

    #[test]
    fn directory() {
        assert!(matches("src/", "src/file.c"));
        assert!(matches("src/", "/home/user/src/file.c"));
        assert!(matches("src/", "/home/user/src/sub/file.c"));
        assert!(matches("src/", "src/"));
        assert!(!matches("src/", "src"));
        assert!(!matches("src/", "src2/"));
    }

    #[test]
    fn question() {
        assert!(matches("fi?e.c", "src/file.c"));
        assert!(!matches("src?file.c", "src/file.c"));
        assert!(!matches("file.c?", "src/file.c"));
    }

    #[test]
    fn star() {
        assert!(matches("*.c", "src/file.c"));
        assert!(matches("src/*.c", "src/file.c"));
        assert!(matches("src/file*", "src/file.c"));
        assert!(matches("src/file.c*", "src/file.c"));
        assert!(matches("fi*le.c", "src/file.c"));
        assert!(matches("f*l*.c", "src/file.c"));
        assert!(matches("f*e*.c", "src/file.c"));
        assert!(!matches("f*f*.c", "src/file.c"));
        assert!(matches("src/*/*.c", "src/sub/file.c"));
        assert!(!matches("src/*.c", "src/sub/file.c"));
    }

    #[test]
    fn utf8() {
        assert!(matches("fi?e.c", "src/fiłe.c"));
        assert!(!matches("fi??e.c", "src/fiłe.c"));
        assert!(matches("fi*e.c", "src/fiłe.c"));
    }

    #[test]
    fn star_star() {
        assert!(matches("src/**.c", "src/file.c"));
        assert!(!matches("src/**.c", "src/sub/file.c"));
        // Trailing `/**` means directory.
        assert!(matches("src/**", "src/file.c"));
        assert!(matches("src/**", "src/sub/file.c"));
        assert!(!matches("src/**", "src"));
        // Trailing `/**/` also means directory.
        assert!(matches("src/**/", "src/file.c"));
        assert!(matches("src/**/", "src/sub/file.c"));
        assert!(!matches("src/**/", "src"));
        // Leading `**/` is ignored.
        assert!(matches("**/file.c", "file.c"));
        assert!(matches("**/file.c", "src/file.c"));
        assert!(matches("**/src/", "/src/file.c"));
    }

    #[test]
    fn slash_star_star_slash() {
        assert!(matches("src/**/file.c", "src/file.c"));
        assert!(matches("src/**/file.c", "src/sub/file.c"));
        assert!(matches("src/**/file.c", "src/sub/sub/file.c"));
        assert!(!matches("src/**/file.c", "src/subfile.c"));
        assert!(matches("src/**/*.c", "src/file.c"));
        assert!(matches("src/**/*.c", "src/sub/file.c"));
        assert!(!matches("src/**/*.c", "srcfile.c"));
        assert!(matches("s*c/**/file.c", "src/sub/file.c"));
        assert!(matches("src/**/fi?e.c", "src/sub/file.c"));
        assert!(matches("src/**/*b/file.c", "src/ab/b/file.c"));
        assert!(matches("src/**/sub/**/file.c", "src/sub/file.c"));
        assert!(matches("src/**/sub/**/file.c", "src/a/sub/b/file.c"));
    }
}
