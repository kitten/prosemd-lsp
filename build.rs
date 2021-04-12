use flate2::{write::GzEncoder, Compression};
use std::env;
use std::io::BufReader;
use std::path::PathBuf;
use xz2::bufread::{XzDecoder, XzEncoder};

fn build_treesitter() {
  println!("cargo:rerun-if-changed=vendor/tree-sitter-markdown/parser.c");
  println!("cargo:rerun-if-changed=vendor/tree-sitter-markdown/scanner.cc");

  let md_dir: PathBuf = ["vendor", "tree-sitter-markdown"].iter().collect();

  cc::Build::new()
    .include(&md_dir)
    .file(md_dir.join("parser.c"))
    .warnings(false)
    .compile("tree-sitter-markdown-parser");
  cc::Build::new()
    .include(&md_dir)
    .file(md_dir.join("scanner.cc"))
    .warnings(false)
    .cpp(true)
    .compile("tree-sitter-markdown-scanner");
}

fn build_nlprule_binary() -> std::result::Result<(), Box<(dyn std::error::Error + 'static)>> {
  let out = env::var("OUT_DIR").expect("OUT_DIR exists in env vars. qed");
  let out = PathBuf::from(out);

  println!("cargo:rerun-if-changed=vendor/nlprule-data/en_rules.bin.xz");
  println!("cargo:rerun-if-changed=vendor/nlprule-data/en_tokenizer.bin.xz");
  println!("cargo:rerun-if-changed={}/en_rules.bin.gz", out.display());
  println!(
    "cargo:rerun-if-changed={}/en_tokenizer.bin.gz",
    out.display()
  );

  let cwd = env::current_dir().expect("Current dir must exist. qed");

  let cache_dir = Some(cwd.join("vendor/nlprule-data"));

  nlprule_build::BinaryBuilder::new(&["en"], &out)
    .fallback_to_build_dir(false)
    .cache_dir(cache_dir)
    .transform(
      |source, mut sink| {
        let mut encoder = XzEncoder::new(BufReader::new(source), 9);
        std::io::copy(&mut encoder, &mut sink)?;
        Ok(())
      },
      |mut path: PathBuf| -> Result<PathBuf, Box<(dyn std::error::Error + Send + Sync + 'static)>> {
        path.set_extension("bin.xz");
        Ok(path)
      },
    )
    .build()?
    .postprocess(
      |source, sink| {
        let mut decoder = XzDecoder::new(BufReader::new(source));
        let mut encoder = GzEncoder::new(sink, Compression::default());
        std::io::copy(&mut decoder, &mut encoder)?;
        Ok(())
      },
      |mut path: PathBuf| -> PathBuf {
        path.set_extension("gz");
        path
      },
    )?;

  Ok(())
}

fn main() -> std::result::Result<(), Box<(dyn std::error::Error + 'static)>> {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=Cargo.toml");

  build_treesitter();
  build_nlprule_binary()?;

  Ok(())
}
