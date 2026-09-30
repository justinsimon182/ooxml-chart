//! Syntax check for the A1-style references a chart holds in `<c:f>`.
//!
//! The goal is to catch what Excel would repair or refuse (a missing `!`, a
//! stray colon, a column past XFD) without turning away anything it accepts:
//! quoted and unquoted sheet names, a workbook index (`[1]Sheet1!A1`), cells,
//! ranges, whole columns and rows, defined names, and a parenthesised union of
//! those. It checks shape, not whether the sheet or name exists.

use crate::error::ChartError;

const MAX_COLUMN: u32 = 16_384; // XFD
const MAX_ROW: u32 = 1_048_576;
const MAX_SHEET_NAME: usize = 31;
const MAX_NAME: usize = 255;

/// Checks one reference, returning [`ChartError::InvalidReference`] if it
/// would not parse.
pub(crate) fn check(reference: &str) -> Result<(), ChartError> {
    parse(reference).map_err(|reason| ChartError::InvalidReference {
        reference: reference.to_string(),
        reason,
    })
}

fn parse(reference: &str) -> Result<(), &'static str> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err("it is empty");
    }
    if let Some(inner) = reference
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        // A union of areas: `(Sheet1!$A$1:$A$3,Sheet1!$C$1:$C$3)`.
        let mut any = false;
        for part in split_top_level(inner)? {
            any = true;
            area_with_sheet(part.trim())?;
        }
        return if any {
            Ok(())
        } else {
            Err("the union is empty")
        };
    }
    area_with_sheet(reference)
}

/// Splits on commas that are outside single quotes.
fn split_top_level(text: &str) -> Result<Vec<&str>, &'static str> {
    let mut parts = Vec::new();
    let (mut start, mut quoted) = (0, false);
    for (index, ch) in text.char_indices() {
        match ch {
            '\'' => quoted = !quoted,
            ',' if !quoted => {
                parts.push(&text[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if quoted {
        return Err("a quoted sheet name is never closed");
    }
    parts.push(&text[start..]);
    Ok(parts)
}

fn area_with_sheet(text: &str) -> Result<(), &'static str> {
    if text.is_empty() {
        return Err("a part of the union is empty");
    }
    let area = if let Some(quoted) = text.strip_prefix('\'') {
        let (name, rest) = split_quoted(quoted)?;
        sheet_name(&name, true)?;
        rest.strip_prefix('!')
            .ok_or("a quoted sheet name must be followed by `!`")?
    } else if let Some((name, area)) = text.split_once('!') {
        sheet_name(name, false)?;
        area
    } else {
        text
    };
    area_part(area)
}

/// Reads up to the closing quote (`''` is an escaped apostrophe), returning the
/// decoded name and what follows the quote.
fn split_quoted(text: &str) -> Result<(String, &str), &'static str> {
    let mut name = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch != '\'' {
            name.push(ch);
        } else if matches!(chars.peek(), Some((_, '\''))) {
            name.push('\'');
            chars.next();
        } else {
            return Ok((name, &text[index + 1..]));
        }
    }
    Err("a quoted sheet name is never closed")
}

fn sheet_name(name: &str, quoted: bool) -> Result<(), &'static str> {
    // An external workbook is written as a leading `[index]`.
    let name = match name.strip_prefix('[') {
        Some(rest) => {
            let (index, after) = rest
                .split_once(']')
                .ok_or("a workbook index is never closed")?;
            if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
                return Err("a workbook index must be a number in brackets");
            }
            after
        }
        None => name,
    };
    if name.is_empty() {
        return Err("the sheet name is empty");
    }
    if name.chars().count() > MAX_SHEET_NAME {
        return Err("a sheet name is at most 31 characters");
    }
    if name
        .chars()
        .any(|c| c.is_control() || matches!(c, '\\' | '/' | '?' | '*' | '[' | ']' | ':'))
    {
        return Err("a sheet name cannot contain any of \\ / ? * [ ] :");
    }
    if !quoted
        && name
            .chars()
            .any(|c| !(c.is_alphanumeric() || matches!(c, '_' | '.')))
    {
        return Err("a sheet name with spaces or punctuation must be in single quotes");
    }
    Ok(())
}

#[derive(PartialEq)]
enum Token {
    Cell,
    Column,
    Row,
    Other,
}

/// What a half of an area looks like, and an error if it is shaped like a cell,
/// column or row but reaches past the sheet.
fn classify(text: &str) -> Result<Token, &'static str> {
    let rest = text.strip_prefix('$').unwrap_or(text);
    let letters = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
    let (column, after) = rest.split_at(letters);
    let after_dollar = after.strip_prefix('$');
    let digits_text = after_dollar.unwrap_or(after);
    let digits = digits_text.bytes().take_while(u8::is_ascii_digit).count();
    if digits != digits_text.len() {
        return Ok(Token::Other);
    }
    let row = |text: &str| -> Result<(), &'static str> {
        match text.parse::<u32>() {
            Ok(1..=MAX_ROW) => Ok(()),
            _ => Err("a row is from 1 to 1048576"),
        }
    };
    let column_ok = |text: &str| -> Result<(), &'static str> {
        // Past three letters is past XFD; stop before the sum can overflow.
        let number = text.bytes().take(4).fold(0u32, |n, b| {
            n * 26 + u32::from(b.to_ascii_uppercase() - b'A' + 1)
        });
        if text.len() <= 3 && (1..=MAX_COLUMN).contains(&number) {
            Ok(())
        } else {
            Err("a column is from A to XFD")
        }
    };
    match (
        column.is_empty(),
        digits_text.is_empty(),
        after_dollar.is_some(),
    ) {
        // `A1`, `$A$1`, `$A1`, `A$1`
        (false, false, _) => column_ok(column)
            .and_then(|()| row(digits_text))
            .map(|()| Token::Cell),
        // `A` or `$A`: a column (only a range half, or a name when alone)
        (false, true, false) => column_ok(column).map(|()| Token::Column),
        // `1` or `$1`
        (true, false, false) => row(digits_text).map(|()| Token::Row),
        _ => Ok(Token::Other),
    }
}

fn area_part(area: &str) -> Result<(), &'static str> {
    if area.is_empty() {
        return Err("there is nothing after the sheet name");
    }
    let mut halves = area.split(':');
    let first = halves.next().unwrap_or_default();
    match (halves.next(), halves.next()) {
        (_, Some(_)) => Err("a range has one colon"),
        (None, None) => match classify(first) {
            Ok(Token::Cell) => Ok(()),
            // `A`, or a column-shaped token past XFD such as `XYZ100`, reads as
            // a defined name, as it does in Excel.
            Ok(Token::Column) | Ok(Token::Other) | Err(_) => defined_name(first),
            Ok(Token::Row) => Err("a lone row number is not a reference"),
        },
        (Some(second), None) => {
            let (a, b) = (classify(first)?, classify(second)?);
            match (a, b) {
                (Token::Cell, Token::Cell)
                | (Token::Column, Token::Column)
                | (Token::Row, Token::Row) => Ok(()),
                _ => Err("a range joins two cells, two columns or two rows"),
            }
        }
    }
}

fn defined_name(name: &str) -> Result<(), &'static str> {
    let mut chars = name.chars();
    let first = chars.next().ok_or("the name is empty")?;
    if !(first.is_alphabetic() || matches!(first, '_' | '\\')) {
        return Err("it is neither a cell, a range nor a defined name");
    }
    if name.chars().count() > MAX_NAME
        || !chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '\\' | '?'))
    {
        return Err("it is neither a cell, a range nor a defined name");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn what_excel_writes_is_accepted() {
        for reference in [
            "Sheet1!$A$1",
            "Sheet1!$A$2:$A$5",
            "'Sheet1'!$B$2:$B$5",
            "'My Sheet'!A1:B2",
            "'Bob''s data'!$A$1",
            "'Q1 (2024)'!$C$3",
            "Sheet1!$A:$B",
            "Sheet1!$3:$5",
            "'Sheet 1'!A1",
            "[1]Sheet1!$A$1:$A$9",
            "'[2]Sheet 1'!$A$1",
            "(Sheet1!$A$1:$A$3,Sheet1!$C$1:$C$3)",
            "('My Sheet'!$A$1,'My Sheet'!$C$1)",
            "Sheet1!MyRange",
            "Sheet1!_data.2024",
            "MyRange",
            "Sheet1!XFD1048576",
            "Sheet1!B2:A1",
            "Sheet1!XYZ100",
            "Sheet1!A0",
            "Sheet1!A1048577",
            "A1",
            "a",
            "Données!$A$1",
            "  Sheet1!$A$1  ",
        ] {
            assert_eq!(parse(reference), Ok(()), "{reference}");
        }
    }

    #[test]
    fn what_excel_would_repair_is_refused() {
        for reference in [
            "",
            "   ",
            "Sheet1!",
            "!$A$1",
            "Sheet1$A$1",
            "Sheet1!A1:",
            "Sheet1!:A1",
            "Sheet1!A1:B2:C3",
            "Sheet1!A:1",
            "Sheet1!1",
            "Sheet1!A1:B",
            "Sheet1!XFE1:XFE2",
            "Sheet1!A0:B2",
            "Sheet1!A1:A1048577",
            "Sheet1!$XYZ100",
            "Sheet1!A:XFE",
            "'Sheet1!$A$1",
            "'Sheet1'$A$1",
            "''!$A$1",
            "Bad Name!$A$1",
            "Sheet-1!$A$1",
            "'a/b'!$A$1",
            "'a:b'!$A$1",
            "'0123456789012345678901234567890123'!$A$1",
            "[x]Sheet1!A1",
            "[1Sheet1!A1",
            "()",
            "(Sheet1!A1,)",
            "(Sheet1!A1,Sheet1!B1",
            "Sheet1!$A$1 $B$2",
            "Sheet1!A$",
            "Sheet1!$$A1",
            "Sheet1!5A",
        ] {
            assert!(parse(reference).is_err(), "{reference:?} should be refused");
        }
    }
}
