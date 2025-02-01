use crate::server::api::traits::{RequestHandler, SyncRequestHandler};
use crate::server::api::ResultExt;
use crate::server::client::Requester;
use crate::server::{client::Notifier, Result};
use crate::Session;
use lsp_types::{self as types, request as req, HoverParams, TextDocumentItem};

pub(crate) struct Hover;

impl RequestHandler for Hover {
    type RequestType = req::HoverRequest;
}

impl SyncRequestHandler for Hover {
    fn run(
        session: &mut Session,
        notifier: Notifier,
        _requester: &mut Requester,
        params: HoverParams,
    ) -> Result<Option<types::Hover>> {
        let url = &params.text_document_position_params.text_document.uri;
        session
            .ensure_project_db_for_baml_file(url)
            .internal_error()?;
        let project = session
            .default_project_db_mut()
            .expect("Ensured that a project db exists");
        let text_document_item = match project.baml_project.files.get(&url.to_string()) {
            None => {
                tracing::warn!("Failed to find doc {:?}", url);
                Err(anyhow::anyhow!(
                    "File {} was not present in the project",
                    url
                ))
            }
            Some(contents) => Ok(TextDocumentItem {
                uri: url.clone(),
                language_id: "BAML".to_string(),
                text: contents.clone(),
                version: 1,
            }),
        }
        .internal_error()?;
        let position = params.text_document_position_params.position;
        let hover = project
            .handle_hover_request(&text_document_item, &position, notifier)
            .internal_error()?;
        Ok(hover)
    }
}
