use crate::{
  node_util,
  parser::parser,
  validator::{TextSuggestion, Validator},
};

use lspower::lsp::{
  CodeAction, CodeActionKind, Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range,
  TextEdit, WorkspaceEdit,
};

use std::{
  collections::hash_map::HashMap,
  sync::{Arc, Mutex},
};
use tree_sitter::{InputEdit, Parser, Point, Tree};
use url::Url;
use xi_rope::{rope::Utf16CodeUnitsMetric, Interval, Rope};

#[derive(Clone)]
pub struct Document {
  version: i32,
  parser: Arc<Mutex<Parser>>,
  rope: Rope,
  tree: Tree,
}

unsafe impl Send for Document {}
unsafe impl Sync for Document {}

impl Document {
  pub fn new(text: String) -> Self {
    let mut parser = parser();
    let tree = parser.parse(&text, None).unwrap();
    let rope = Rope::from(text);
    let parser = Arc::new(Mutex::new(parser));
    Self {
      version: 0,
      rope,
      parser,
      tree,
    }
  }

  pub fn version(&self) -> i32 {
    self.version
  }

  pub fn edit(
    &self,
    version: i32,
    edits: impl Iterator<Item = (Option<Range>, String)>,
  ) -> Document {
    edits.fold(self.clone(), |doc, (range, text)| match range {
      Some(range) => edit_range(&doc, version, range, text),
      None => edit_fulltext(&doc, version, text),
    })
  }

  pub fn diagnostics(&self, validator: &Validator) -> Vec<Diagnostic> {
    node_util::find_text_nodes(&self.tree)
      .iter()
      .flat_map(|node| validator.suggest(&node_util::get_node_text(&self.rope, &node)))
      .map(|suggestion| suggestion_to_diagnostic(&self.rope, &validator, suggestion))
      .collect::<Vec<Diagnostic>>()
  }

  pub fn actions(&self, uri: &Url, diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
    diagnostics
      .iter()
      .filter_map(|diagnostic| {
        diagnostic_to_edits(&self.rope, &diagnostic).map(|edits| {
          let title = format!(
            "Autofix {}",
            diagnostic
              .source
              .as_ref()
              .unwrap_or(&"suggestion".to_string())
          );
          let mut changes = HashMap::new();
          changes.insert(uri.clone(), edits);
          CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            is_preferred: Some(true),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit::new(changes)),
            disabled: None,
            command: None,
            data: None,
          }
        })
      })
      .collect()
  }
}

fn diagnostic_to_edits(rope: &Rope, diagnostic: &Diagnostic) -> Option<Vec<TextEdit>> {
  diagnostic
    .data
    .as_ref()
    .map(|data| serde_json::from_value::<TextSuggestion>(data.clone()).ok())
    .flatten()
    .map(|edit| {
      edit
        .replacements
        .into_iter()
        .map(|replacement| {
          let range = Range::new(
            offset_to_position(&rope, replacement.start),
            offset_to_position(&rope, replacement.end),
          );
          TextEdit::new(range, replacement.replacement)
        })
        .collect()
    })
}

fn suggestion_to_diagnostic(
  rope: &Rope,
  validator: &Validator,
  suggestion: TextSuggestion,
) -> Diagnostic {
  let range = Range::new(
    offset_to_position(&rope, suggestion.start),
    offset_to_position(&rope, suggestion.end),
  );

  let mut diagnostic = Diagnostic {
    range,
    severity: Some(DiagnosticSeverity::Warning),
    message: suggestion.message.clone(),
    related_information: None,
    data: Some(serde_json::to_value(&suggestion).unwrap()),
    ..Diagnostic::default()
  };

  if let Some(rule) = validator.get_rule(&suggestion.source) {
    diagnostic.code = Some(NumberOrString::String(rule.name().to_string()));
    if let Some(category_id) = rule.category_type() {
      diagnostic.source = Some(category_id.to_string());
      diagnostic.severity = Some(match category_id {
        "grammar" => DiagnosticSeverity::Error,
        "inconsistency" => DiagnosticSeverity::Error,
        "misspelling" => DiagnosticSeverity::Warning,
        "typographical" => DiagnosticSeverity::Warning,
        _other => DiagnosticSeverity::Warning,
      });
    }
  }

  diagnostic
}

fn edit_range(doc: &Document, version: i32, range: Range, text: String) -> Document {
  let start = position_to_offset(&doc.rope, range.start);
  let end = position_to_offset(&doc.rope, range.end);
  let new_end_byte = start + text.as_bytes().len();

  let mut new_rope = doc.rope.clone();
  let mut new_tree = doc.tree.clone();

  new_rope.edit(Interval { start, end }, text);
  new_tree.edit(&InputEdit {
    start_byte: start,
    old_end_byte: end,
    new_end_byte,
    start_position: offset_to_point(&doc.rope, end),
    old_end_position: offset_to_point(&doc.rope, end),
    new_end_position: offset_to_point(&new_rope, new_end_byte),
  });

  let new_tree = doc
    .parser
    .lock()
    .unwrap()
    .parse_with(
      &mut |offset, _pos| get_chunk(&new_rope, offset),
      Some(&new_tree),
    )
    .unwrap();

  Document {
    version,
    parser: doc.parser.clone(),
    rope: new_rope,
    tree: new_tree,
  }
}

fn edit_fulltext(doc: &Document, version: i32, text: String) -> Document {
  let rope = Rope::from(text.clone());
  let tree = doc.parser.lock().unwrap().parse(&text, None).unwrap();
  Document {
    version,
    parser: doc.parser.clone(),
    rope,
    tree,
  }
}

fn position_to_offset(rope: &Rope, pos: Position) -> usize {
  let line_offset = rope.offset_of_line(pos.line as usize);
  let line_slice = rope.slice(line_offset..);
  let char_offset = line_slice.count_base_units::<Utf16CodeUnitsMetric>(pos.character as usize);
  line_offset + char_offset
}

fn offset_to_point(rope: &Rope, offset: usize) -> Point {
  let row = rope.line_of_offset(offset);
  let column = offset - rope.offset_of_line(row);
  Point { row, column }
}

fn offset_to_position(rope: &Rope, offset: usize) -> Position {
  let row = rope.line_of_offset(offset);
  let column = offset - rope.offset_of_line(row);
  Position::new(row as u32, column as u32)
}

fn get_chunk(rope: &Rope, offset: usize) -> &str {
  let cursor = xi_rope::Cursor::new(&rope, offset);
  if let Some((node, idx)) = cursor.get_leaf() {
    &node[idx..]
  } else {
    ""
  }
}
