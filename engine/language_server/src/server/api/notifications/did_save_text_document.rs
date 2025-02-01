use crate::server::api::ResultExt;
use crate::server::client::{Notifier, Requester};
use crate::server::Result;
use crate::session::Session;
use lsp_types as types;
use lsp_types::notification as notif;

pub struct DidSaveTextDocument;

impl super::NotificationHandler for DidSaveTextDocument {
    type NotificationType = notif::DidSaveTextDocument;
}

impl super::SyncNotificationHandler for DidSaveTextDocument {
    fn run(
        session: &mut Session,
        notifier: Notifier,
        _requester: &mut Requester,
        params: types::DidSaveTextDocumentParams,
    ) -> Result<()> {
        session.reload(Some(notifier.clone())).internal_error()?;
        tracing::info!("About to run generator");
        session
            .ensure_project_db_for_baml_file(&params.text_document.uri)
            .internal_error()?;
        session
            .default_project_db_mut()
            .expect("Ensured that a project db exists")
            .run_generators_without_debounce(
                |_| {
                    notifier
                        .notify_baml_info(&format!(
                            "BAML: Client generated! (Using installed baml-cli {})",
                            env!("CARGO_PKG_VERSION") // TODO: Use baml-cli version.
                        ))
                        .unwrap_or(())
                },
                |e| {
                    notifier
                        .notify_baml_error(&format!("Error generating: {e}"))
                        .unwrap_or(())
                },
            );
        Ok(())
    }
}
