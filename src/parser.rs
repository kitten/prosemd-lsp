use tree_sitter::{Language, Parser};

extern "C" {
  fn tree_sitter_markdown() -> Language;
}

pub fn parser() -> Parser {
  let language = unsafe { tree_sitter_markdown() };
  let mut parser = Parser::new();
  parser.set_language(language).unwrap();
  parser
}
