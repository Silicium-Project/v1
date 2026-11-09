use super::*;

#[test]
fn reads_blocks_words_lists_comments_and_keyframes() {
    let text = "# scene\nrender : { samples = 16 mode = path } // end\n\
                primitives : ( { name = \"car body\" } );\n\
                mesh = { vertices = { 0 0 0  1e-3 -2 +3 } }\n\
                x = { 0 = 1  10 = 3 }\n";
    let entries = parse(text).unwrap();
    let render = entries.block("render").unwrap();
    assert_eq!(render.number("samples", 1.0, 0.0), 16.0);
    assert_eq!(render.word("mode"), Some("path"));
    let primitives = entries.block("primitives").unwrap();
    let Value::Block(inner) = &primitives[0].value else {
        panic!("bare block expected")
    };
    assert_eq!(inner.word("name"), Some("car body"));
    let mesh = entries.block("mesh").unwrap();
    assert_eq!(
        mesh.block("vertices").unwrap().numbers(),
        vec![0.0, 0.0, 0.0, 1e-3, -2.0, 3.0]
    );
    assert_eq!(entries.number("x", 0.0, -5.0), 1.0);
    assert_eq!(entries.number("x", 0.0, 5.0), 2.0);
    assert_eq!(entries.number("x", 0.0, 50.0), 3.0);
    assert_eq!(entries.entry("x").unwrap().line, 5);
}

#[test]
fn reports_unbalanced_braces_with_their_line() {
    assert_eq!(parse("a = {\nb = 1\n").unwrap_err().line, 1);
    assert_eq!(parse("a = 1\n}\n").unwrap_err().line, 2);
    assert_eq!(parse("a = \"oops\n").unwrap_err().line, 1);
}
