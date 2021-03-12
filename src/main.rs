mod backend;
mod document;
mod node_util;
mod parser;
mod validator;

use crate::backend::Backend;
use async_std::{io::*, net};
use lspower::{LspService, Server};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Options {
  #[structopt(short, long)]
  version: bool,

  #[structopt(long)]
  stdio: bool,

  #[structopt(long)]
  socket: Option<u32>,
}

#[async_std::main]
async fn main() -> Result<()> {
  env_logger::init();

  let options = Options::from_args();
  if options.version {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    println!("{}: v{}", name, version);
    return Ok(());
  }

  let (service, messages) = LspService::new(Backend::new);
  if options.stdio {
    let input = stdin();
    let output = stdout();
    Server::new(input, output)
      .interleave(messages)
      .serve(service)
      .await;
    Ok(())
  } else if let Some(port) = options.socket {
    let listener = net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    let (stream, _) = listener.accept().await?;
    let input = BufReader::new(&stream);
    let output = BufWriter::new(&stream);
    Server::new(input, output)
      .interleave(messages)
      .serve(service)
      .await;
    Ok(())
  } else {
    Err(Error::new(
      ErrorKind::Other,
      "prosemd-lsp needs --stdio or --socket options to listen to",
    ))
  }
}
