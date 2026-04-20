//! # Property value parsers
//!
//! This module contains all property value parsers needed to parse
//! MML message bodies: [val], [quoted_val] and
//! [maybe_quoted_const_val].

use crate::message::body::{compiler::tokens::Val, BACKSLASH, DOUBLE_QUOTE, GREATER_THAN, SPACE};

use super::prelude::*;

/// Control characters that must never appear in a property value,
/// whether quoted or not. CR / LF would otherwise splice into MIME
/// header output as a new header line (header-injection), and NUL
/// terminates C-string-backed transports. Rejecting these at parse
/// time keeps every prop's emission path safe regardless of whether
/// it eventually writes through `mail_builder::Raw::new` (verbatim)
/// or a structured header wrapper.
const FORBIDDEN_CTRL: [char; 3] = ['\r', '\n', '\0'];

/// The property value parser.
///
/// It parses all characters except the backslack, the space and the
/// greater-than characters. They still can be parsed by escaping them
/// with a backslash. CR / LF / NUL are rejected outright — even
/// escaped — because there is no legitimate reason to embed them
/// in an MML attribute value and allowing them would let a caller
/// inject arbitrary MIME headers via `Raw`-emitted props.
pub(crate) fn val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = [BACKSLASH, SPACE, GREATER_THAN];

    choice((
        backslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars).and_is(one_of(FORBIDDEN_CTRL).not()),
    ))
    .repeated()
    .at_least(1)
    .collect()
}

/// The quoted property value parser.
///
/// It parses all characters except the backslack and the double quote
/// characters. They still can be parsed by escaping them with a
/// backslack. CR / LF / NUL are rejected here for the same reason
/// as [`val`] — see its doc comment.
pub(crate) fn quoted_val<'a>() -> impl Parser<'a, &'a str, Val<'a>, ParserError<'a>> + Clone {
    let escapable_chars = [BACKSLASH, DOUBLE_QUOTE];

    choice((
        backslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars).and_is(one_of(FORBIDDEN_CTRL).not()),
    ))
    .repeated()
    .to_slice()
    .delimited_by(dquote(), dquote())
}

/// The maybe quoted const property value parser.
///
/// It parses either the given const value or the quoted version of
/// it.
pub(crate) fn maybe_quoted_const_val(
    val: &str,
) -> impl Parser<'_, &str, Val<'_>, ParserError<'_>> + Clone {
    choice((
        just(val).to_slice().delimited_by(dquote(), dquote()),
        just(val).to_slice(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn val() {
        assert_eq!(
            super::val().parse("value").into_result(),
            Ok("value".into())
        );

        assert_eq!(
            super::val().parse("escaped\\ space\"").into_result(),
            Ok("escaped space\"".into()),
        );

        // example from the Emacs MML module:
        assert_eq!(
            super::val().parse("/home/user/#hello$^yes").into_result(),
            Ok("/home/user/#hello$^yes".into()),
        );
    }

    #[test]
    fn quoted_val() {
        assert_eq!(super::quoted_val().parse("\"\"").into_result(), Ok(""));

        assert_eq!(
            super::quoted_val().parse("\"quoted val\"").into_result(),
            Ok("quoted val"),
        );

        assert_eq!(
            super::quoted_val()
                .parse("\"\\\\quoted \\\"val\\\"\"")
                .into_result(),
            Ok("\\\\quoted \\\"val\\\""),
        );
    }

    #[test]
    fn val_rejects_control_chars() {
        // CR / LF / NUL must not be accepted even mid-value — they
        // would splice into Raw-emitted headers as a new header line.
        assert!(super::val().parse("foo\r\nbar").has_errors());
        assert!(super::val().parse("foo\nbar").has_errors());
        assert!(super::val().parse("foo\rbar").has_errors());
        assert!(super::val().parse("foo\0bar").has_errors());
    }

    #[test]
    fn quoted_val_rejects_control_chars() {
        // Same rule for quoted values — the dquote delimiters are
        // not a header-injection barrier once Raw::new sees the
        // bytes.
        assert!(super::quoted_val().parse("\"foo\r\nbar\"").has_errors());
        assert!(super::quoted_val().parse("\"foo\nbar\"").has_errors());
        assert!(super::quoted_val().parse("\"foo\rbar\"").has_errors());
        assert!(super::quoted_val().parse("\"foo\0bar\"").has_errors());
    }

    #[test]
    fn maybe_quoted_val() {
        assert_eq!(
            super::maybe_quoted_const_val("key")
                .parse("key")
                .into_result(),
            Ok("key")
        );

        assert_eq!(
            super::maybe_quoted_const_val("key")
                .parse("\"key\"")
                .into_result(),
            Ok("key")
        );
    }
}
