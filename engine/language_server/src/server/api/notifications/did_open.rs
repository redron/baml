use lsp_types::notification::DidOpenTextDocument;
use lsp_types::{DidOpenTextDocumentParams, PublishDiagnosticsParams};

use crate::server::api::diagnostics::session_lsp_diagnostics;
use crate::server::api::traits::{NotificationHandler, SyncNotificationHandler};
use crate::server::api::ResultExt;
use crate::server::client::{Notifier, Requester};
use crate::server::Result;
use crate::session::Session;

pub(crate) struct DidOpenTextDocumentHandler;

impl NotificationHandler for DidOpenTextDocumentHandler {
    type NotificationType = DidOpenTextDocument;
}

impl SyncNotificationHandler for DidOpenTextDocumentHandler {
    fn run(
        session: &mut Session,
        notifier: Notifier,
        _requester: &mut Requester,
        params: DidOpenTextDocumentParams,
    ) -> Result<()> {
        tracing::info!("DidOpenTextDocumentHandler");
        // let Ok(path) = url_to_any_system_path(&params.text_document.uri) else {
        //     return Ok(());
        // };

        // let document = TextDocument::new(params.text_document.text, params.text_document.version);

        let url = params.text_document.uri;
        // session.open_text_document(url.clone(), document);
        session
            .ensure_project_db_for_baml_file(&url)
            .internal_error()?;
        session.reload(Some(notifier.clone())).internal_error()?;

        let diagnostics = session_lsp_diagnostics(session, &url);

        notifier
            .notify::<lsp_types::notification::PublishDiagnostics>(PublishDiagnosticsParams {
                uri: url,
                version: Some(params.text_document.version),
                diagnostics,
            })
            .expect("TODO");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_to_url() {
        let path_buf = PathBuf::from("file:///test.baml");
        let url = Url::from_file_path(&path_buf).unwrap();
        assert_eq!(url.as_str(), "file:///test.baml");
    }

    #[test]
    fn parse_path_with_file_prefix() {
        let url = Url::parse("file:///test.baml").unwrap();
        assert_eq!(url.as_str(), "file:///test.baml");
    }
}
