use tree_sitter::{Node, Tree};
use xi_rope::Rope;

pub fn find_text_nodes(tree: &Tree) -> Vec<Node> {
  let mut cursor = tree.walk();
  let mut nodes = Vec::new();
  let mut recurse = true;

  loop {
    if (recurse && cursor.goto_first_child()) || cursor.goto_next_sibling() {
      recurse = true;
    } else if cursor.goto_parent() {
      recurse = false;
    } else {
      break;
    }

    let node = cursor.node();
    let kind_id = node.kind_id();
    if kind_id == 124 /* paragraph */
      || kind_id == 169 /* heading content */
      || kind_id == 235
    /* table cell */
    {
      nodes.push(node);
      recurse = false;
    } else if kind_id == 129
    /* frontmatter heading */
    {
      recurse = false;
    }
  }

  nodes
}

pub struct TextChunk {
  pub clean_length: usize,
  pub start: usize,
  pub end: usize,
}

pub struct TextRange {
  pub clean_text: String,
  pub chunks: Vec<TextChunk>,
}

pub fn get_node_text(rope: &Rope, node: &Node) -> TextRange {
  let mut cursor = node.walk();
  let mut text = String::new();
  let mut chunks: Vec<TextChunk> = Vec::new();
  let mut depth = 0;
  let mut recurse = true;

  while depth >= 0 {
    if recurse && cursor.goto_first_child() {
      recurse = true;
      depth += 1;
    } else if depth > 0 && cursor.goto_next_sibling() {
      recurse = true;
    } else if depth > 0 && cursor.goto_parent() {
      recurse = false;
      depth -= 1;
      continue;
    } else {
      break;
    }

    let node = cursor.node();
    match node.kind_id() {
      190 /* link destination */
      | 230 /* image description */
      | 136 /* fenced code block */
      | 134 /* indented code block */
      | 201 /* html opening tag */
      | 202 /* html self-closing tag */
      | 185 /* image */ => {
        recurse = false;
      },
      211 /* text */ => {
        let start = node.start_byte();
        let end = node.end_byte();
        let length = end - start;
        let slice = rope.slice_to_cow(start..end).to_mut().clone();
        text.push_str(&slice);
        chunks.push(TextChunk { clean_length: length, start, end });
      },
      111 /* soft line break */ => {
        let start = node.start_byte();
        let end = node.end_byte();
        text.push(' ');
        chunks.push(TextChunk { clean_length: 1, start, end });
      },
      200 /* code span */ => {
        let start = node.start_byte();
        let end = node.end_byte();
        text.push_str("[code]");
        chunks.push(TextChunk { clean_length: 6, start, end });
        recurse = false;
      }
      _other => ()
    };
  }

  TextRange {
    clean_text: text,
    chunks,
  }
}
