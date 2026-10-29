//! Reading back the PDF files the exports write (B2, B4): plain ASCII, uncompressed.
// Test code; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used)]

/// The numbers on a line of drawing commands, in order.
pub fn numbers(line: &str) -> Vec<f64> {
    line.split_whitespace()
        .filter_map(|word| word.parse().ok())
        .collect()
}

/// Checks the file's frame: header and end, `startxref` pointing at the cross-reference
/// table, every entry pointing at its object, the trailer's size, and every stream as
/// long as its dictionary says. Returns the number of objects.
pub fn check_structure(pdf: &str) -> usize {
    assert!(pdf.starts_with("%PDF-1.7\n"));
    assert!(pdf.ends_with("%%EOF\n"));

    let startxref = pdf.rfind("startxref\n").unwrap() + "startxref\n".len();
    let xref: usize = pdf[startxref..].lines().next().unwrap().parse().unwrap();
    assert!(pdf[xref..].starts_with("xref\n"));
    let mut table = pdf[xref..].lines().skip(1);
    let count: usize = table.next().unwrap()["0 ".len()..].parse().unwrap();
    assert_eq!(table.next(), Some("0000000000 65535 f "));
    for object in 1..count {
        let offset: usize = table.next().unwrap()[..10].parse().unwrap();
        assert!(
            pdf[offset..].starts_with(&format!("{object} 0 obj\n")),
            "object {object}"
        );
    }
    assert!(pdf.contains(&format!("/Size {count} /Root 1 0 R")));

    for (at, _) in pdf.match_indices("<< /Length ") {
        let length = numbers(&pdf[at..][..30])[0] as usize;
        let start = at + pdf[at..].find("stream\n").unwrap() + "stream\n".len();
        assert!(
            pdf[start + length..].starts_with("endstream"),
            "stream at {at} isn't {length} bytes long"
        );
    }
    count - 1
}

/// Every content stream's drawing commands, in file order: one per page.
pub fn streams(pdf: &str) -> Vec<&str> {
    pdf.match_indices("stream\n")
        .filter(|(at, _)| !pdf[..*at].ends_with("end"))
        .map(|(at, found)| {
            let start = at + found.len();
            let end = start + pdf[start..].find("endstream").unwrap();
            &pdf[start..end]
        })
        .collect()
}
