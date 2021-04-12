use crate::node_util::{TextChunk, TextRange};

use flate2::read::GzDecoder;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use nlprule::{
  rule::Rule, rules_filename, tokenizer_filename, types::Suggestion, Rules, Tokenizer,
};

static TOKENIZER_GZ: &[u8] = include_bytes!(concat!(
  env!("OUT_DIR"),
  "/",
  tokenizer_filename!("en"),
  ".gz"
));

static RULES_GZ: &[u8] =
  include_bytes!(concat!(env!("OUT_DIR"), "/", rules_filename!("en"), ".gz"));

#[derive(Serialize, Deserialize)]
pub struct Replacement {
  pub replacement: String,
  pub start: usize,
  pub end: usize,
}

#[derive(Serialize, Deserialize)]
pub struct TextSuggestion {
  pub source: String,
  pub message: String,
  pub replacements: Vec<Replacement>,
  pub start: usize,
  pub end: usize,
}

pub struct Validator {
  cache: Arc<Mutex<LruCache<String, Vec<Suggestion>>>>,
  tokenizer: Tokenizer,
  rules: Rules,
}

impl Validator {
  pub fn new() -> Self {
    log::debug!("initializing validator...");
    let cache = Arc::new(Mutex::new(LruCache::new(1000)));
    let tokenizer = Tokenizer::from_reader(&mut GzDecoder::new(TOKENIZER_GZ)).unwrap();
    let rules = Rules::from_reader(&mut GzDecoder::new(RULES_GZ))
      .unwrap()
      .into_iter()
      .filter(|rule| {
        match rule.id().to_string().to_lowercase().as_ref() {
          // informally, "todo" is just as well accepted as "to-do"
          "to_do_hyphen.3" => false,
          "to_do_hyphen.2" => false,
          _other => match rule.category_name().to_string().to_lowercase().as_ref() {
            // Wikipedia's style guide contains a few rules that are too opinionated
            "wikipedia" => false,
            "typography" => false,
            _other => true,
          },
        }
      })
      .collect::<Rules>();
    Self {
      cache,
      tokenizer,
      rules,
    }
  }

  pub fn get_rule(&self, id: &str) -> Option<&Rule> {
    self.rules.rules().iter().find(|x| x.name() == id)
  }

  pub fn suggest(&self, text: &TextRange) -> Vec<TextSuggestion> {
    let mut cache = self.cache.lock().unwrap();
    let suggestions = match cache.get(&text.clean_text) {
      Some(suggestions) => suggestions.clone(),
      None => {
        let suggestions = self.rules.suggest(&text.clean_text, &self.tokenizer);
        cache.put(text.clean_text.clone(), suggestions.clone());
        suggestions
      }
    };

    suggestions
      .into_iter()
      .filter_map(|suggestion| compute_edit(&text, suggestion))
      .collect()
  }
}

fn compute_edit(text: &TextRange, suggestion: Suggestion) -> Option<TextSuggestion> {
  let mut chunks = slice_textchunks_for_suggestion(&text, &suggestion);
  if !chunks.is_empty() {
    let start = chunks.first().unwrap().start;
    let end = chunks.last().unwrap().end;
    let left = &text.clean_text[suggestion.span().start().char..suggestion.span().end().char];
    let right = &suggestion.replacements().first().unwrap().clone();
    let mut diff = diff::chars(left, right);

    let mut replacements: Vec<Replacement> = Vec::new();
    while let Some(chunk) = chunks.pop() {
      let replacement = take_diff_last(&mut diff, chunk.clean_length);
      if !replacement.is_empty() {
        replacements.push(Replacement {
          replacement,
          start: chunk.start,
          end: chunk.end,
        });
      }
    }

    Some(TextSuggestion {
      source: suggestion.source().to_string(),
      message: suggestion.message().to_string(),
      replacements,
      start,
      end,
    })
  } else {
    None
  }
}

fn slice_textchunks_for_suggestion(text: &TextRange, suggestion: &Suggestion) -> Vec<TextChunk> {
  let mut chunks: Vec<TextChunk> = Vec::new();
  let mut length = 0;
  let mut index = -1;

  for chunk in &text.chunks {
    length += chunk.clean_length;
    if length >= suggestion.span().start().char {
      index += 1;

      let mut clean_length = chunk.clean_length;
      let mut start = chunk.start;
      let mut end = chunk.end;

      if index == 0 {
        clean_length = length - suggestion.span().start().char;
        start = chunk.end - clean_length;
      }

      if length >= suggestion.span().end().char {
        let slice = length - suggestion.span().end().char;
        clean_length -= slice;
        end = chunk.end - slice;
        chunks.push(TextChunk {
          clean_length,
          start,
          end,
        });
        break;
      }

      chunks.push(TextChunk {
        clean_length,
        start,
        end,
      });
    }
  }

  chunks
}

fn take_diff_last(diff: &mut Vec<diff::Result<char>>, take_last: usize) -> String {
  let mut chars: Vec<char> = Vec::new();
  let mut taken = 0;
  while taken <= take_last {
    match diff.pop() {
      Some(diff::Result::Right(c)) => {
        // added
        chars.push(c);
      }
      Some(diff::Result::Left(_)) => {
        // removed
        taken += 1;
      }
      Some(diff::Result::Both(c, _)) => {
        // preserved
        taken += 1;
        chars.push(c);
      }
      None => break,
    }
  }

  chars.into_iter().rev().collect()
}
