//! The PDF file round the drawing: numbered objects, the cross-reference table that
//! says where each one starts, and the trailer.

use std::fmt::Write;

/// The whole file: `objects` numbered from 1 in order, object 1 being the catalog, and
/// `info` the number of the document info object.
pub(crate) fn write_file((objects, info): (&[String], usize)) -> String {
    let mut file = "%PDF-1.7\n".to_owned();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(file.len());
        let _ = write!(file, "{} 0 obj\n{object}\nendobj\n", index + 1);
    }
    let xref = file.len();
    let _ = write!(file, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(file, "{offset:010} 00000 n ");
    }
    let _ = write!(
        file,
        "trailer\n<< /Size {} /Root 1 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1,
    );
    file
}
