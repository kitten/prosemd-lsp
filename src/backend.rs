use crate::{document::Document, validator::Validator};

use async_std::{
  channel::{unbounded, Receiver, Sender},
  stream::StreamExt,
  sync::{Arc, RwLock},
};
use im::hashmap::HashMap;
use lspower::{jsonrpc::Result, lsp::*, Client, LanguageServer};
use std::time::Duration;
use url::Url;

enum BackendEvent {
  SendDiagnostics(Url),
}

pub struct Backend {
  client: Arc<Client>,
  validator: Arc<Validator>,
  files: Arc<RwLock<HashMap<Url, Document>>>,
  events: (Sender<BackendEvent>, Receiver<BackendEvent>),
}

impl Backend {
  pub fn new(client: Client) -> Self {
    Self {
      client: Arc::new(client),
      validator: Arc::new(Validator::new()),
      files: Arc::new(RwLock::new(HashMap::new())),
      events: unbounded(),
    }
  }

  async fn send_event(&self, event: BackendEvent) {
    let (sender, _) = &self.events;
    let _ = sender.send(event).await;
  }

  fn events_loop(&self) {
    let mut events = {
      let (_, receiver) = &self.events;
      receiver.clone().throttle(Duration::from_millis(100))
    };

    let client = Arc::clone(&self.client);
    let validator = Arc::clone(&self.validator);
    let files = Arc::clone(&self.files);

    async_std::task::spawn(async move {
      while let Some(event) = events.next().await {
        match event {
          BackendEvent::SendDiagnostics(uri) => {
            if let Some(document) = files.read().await.get(&uri) {
              let diagnostics = document.diagnostics(&validator);
              let version = document.version();
              client
                .publish_diagnostics(uri, diagnostics, Some(version))
                .await;
            }
          }
        }
      }
    });
  }
}

#[lspower::async_trait]
impl LanguageServer for Backend {
  async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    log::debug!("initialize");
    self.events_loop();

    Ok(InitializeResult {
      server_info: None,
      capabilities: ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
          TextDocumentSyncKind::Incremental,
        )),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..ServerCapabilities::default()
      },
    })
  }

  async fn did_open(&self, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    log::debug!("did_open: {}", uri);

    {
      let document = Document::new(params.text_document.text);
      let mut files = self.files.write().await;
      *files = files.update(uri.clone(), document);
    }

    self
      .send_event(BackendEvent::SendDiagnostics(uri.clone()))
      .await;
  }

  async fn did_change(&self, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;
    log::debug!("did_change: {}", uri);

    let version = params.text_document.version;
    let changes = params
      .content_changes
      .into_iter()
      .map(|change| (change.range, change.text));

    {
      let mut files = self.files.write().await;
      *files = files.alter(
        |document| match document {
          None => None,
          Some(document) => {
            let document = document.edit(version, changes);
            Some(document)
          }
        },
        uri.clone(),
      );
    }

    self
      .send_event(BackendEvent::SendDiagnostics(uri.clone()))
      .await;
  }

  async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
    let uri = params.text_document.uri.clone();
    log::debug!("code_action: {}", uri);

    Ok(self.files.read().await.get(&uri).map(|document| {
      document
        .actions(&uri, &params.context.diagnostics)
        .into_iter()
        .map(CodeActionOrCommand::CodeAction)
        .collect()
    }))
  }

  async fn did_close(&self, params: DidCloseTextDocumentParams) {
    log::debug!("did_close: {}", params.text_document.uri);
    let mut files = self.files.write().await;
    *files = files.without(&params.text_document.uri);
  }

  async fn shutdown(&self) -> Result<()> {
    let (sender, _) = &self.events;
    sender.close();
    Ok(())
  }
}
