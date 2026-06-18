use pg_parse::Error;

/// `cursorpos` should point at the first character of the offending token (1-based).
#[test]
fn it_reports_cursorpos_for_a_token_error() {
    // 1-based char positions: C=1 R=2 E=3 A=4 T=5 E=6 (space)=7 T=8 ...
    // so the unexpected token `TABL` begins at position 8.
    let err = pg_parse::parse("CREATE TABL public.foo (id int)").unwrap_err();
    match err {
        Error::ParseError { message, cursorpos } => {
            assert_eq!(message, "syntax error at or near \"TABL\"");
            assert_eq!(cursorpos, 8);
        }
        other => panic!("expected ParseError, got {other:?}"),
    }
}

/// For an unexpected end-of-input, `cursorpos` points one past the last character
/// (i.e. character length + 1), not 0.
#[test]
fn it_reports_cursorpos_at_end_of_input() {
    let sql = "SELECT * FROM"; // 13 characters
    let err = pg_parse::parse(sql).unwrap_err();
    match err {
        Error::ParseError { message, cursorpos } => {
            assert_eq!(message, "syntax error at end of input");
            assert_eq!(cursorpos, sql.chars().count() as i32 + 1);
            assert_eq!(cursorpos, 14);
        }
        other => panic!("expected ParseError, got {other:?}"),
    }
}

/// `cursorpos` is a character offset, not a byte offset: a multi-byte character before
/// the error token must not shift the reported position.
#[test]
fn it_reports_cursorpos_as_character_offset() {
    // `é` is two bytes in UTF-8 but a single character. The unexpected token `x` is the
    // 17th character; as a byte offset it would be the 19th.
    let sql = "SELECT 'éé' FRM x";
    let err = pg_parse::parse(sql).unwrap_err();
    match err {
        Error::ParseError { cursorpos, .. } => {
            assert_eq!(
                cursorpos, 17,
                "should be the character offset, not byte offset"
            );
            // Demonstrate the documented char_index -> byte_offset mapping.
            let char_index = (cursorpos - 1) as usize;
            let byte_offset = sql
                .char_indices()
                .nth(char_index)
                .map(|(byte, _)| byte)
                .unwrap_or(sql.len());
            assert_eq!(&sql[byte_offset..], "x");
        }
        other => panic!("expected ParseError, got {other:?}"),
    }
}
