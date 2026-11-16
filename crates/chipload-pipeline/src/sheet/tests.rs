// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn numbers_read_naturally() {
    let w = Words {
        lang: Language::En,
        units: Units::Mm,
    };
    let read = |value: Value| w.say(&Message::new("sheet-stepover").arg("percent", value));
    assert_eq!(read(num(6.0)), "stepover 6 %");
    assert_eq!(read(num(0.5)), "stepover 0.5 %");
    assert_eq!(read(num(-0.0001)), "stepover 0 %");
    assert_eq!(read(num(12.3456)), "stepover 12.346 %");
    assert_eq!(read(num(3.0_f64.to_radians().to_degrees())), "stepover 3 %");
    assert_eq!(svg_num(-0.001), "0");
}

#[test]
fn html_texts_keep_their_markup_and_write_the_rest_as_ascii() {
    assert_eq!(ascii("6 &times; 4 \u{fc}"), "6 &times; 4 &#xfc;");
}

#[test]
fn text_is_escaped_to_ascii() {
    assert_eq!(
        escape("<a & \"b\"> 'c' \u{e5}\u{b0}"),
        "&lt;a &amp; &quot;b&quot;&gt; &#39;c&#39; &#xe5;&#xb0;"
    );
}

#[test]
fn the_sheet_sits_next_to_the_gcode() {
    assert_eq!(
        sheet_path(Path::new("out/part.nc")),
        Path::new("out/part-sheet.html")
    );
}
