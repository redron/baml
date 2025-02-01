use crate::baml_project::position_utils::get_word_at_position;
use crate::baml_project::trim_line;
use crate::server::api::traits::{RequestHandler, SyncRequestHandler};
use crate::server::api::ResultExt;
use crate::server::client::Requester;
use crate::server::{client::Notifier, Result};
use crate::Session;
use lsp_types::{request, CompletionItem, CompletionList, CompletionParams, CompletionResponse};

pub(crate) struct Completion;

impl RequestHandler for Completion {
    type RequestType = request::Completion;
}

impl SyncRequestHandler for Completion {
    fn run(
        session: &mut Session,
        _notifier: Notifier,
        _requester: &mut Requester,
        params: CompletionParams,
    ) -> Result<Option<lsp_types::CompletionResponse>> {
        let url = params.text_document_position.text_document.uri;
        session
            .ensure_project_db_for_baml_file(&url)
            .internal_error()?;
        let project = session
            .default_project_db()
            .expect("Ensured that a project db exists");
        let doc = project.baml_project.files.get(&url.to_string()).unwrap();
        let word = get_word_at_position(&doc, &params.text_document_position.position);
        let cleaned_word = trim_line(&word);
        // let cleaned_word = word;
        tracing::info!("Cleaned word: {:?}", cleaned_word);
        let completions = match cleaned_word.as_str() {
            "_." => Some(vec![
                r#"role("system")"#,
                r#"role("assistant")"#,
                r#"role("user")"#,
            ]),
            "ctx." => Some(vec![r#"output_format"#, r#"client"#]),
            "ctx.client." => Some(vec![r#"name"#, r#"provider"#]),
            _ => None,
        };
        Ok(completions.map(|completions| {
            let completion_list = CompletionList {
                is_incomplete: false,
                items: completions
                    .into_iter()
                    .map(|completion| CompletionItem {
                        label: completion.to_string(),
                        ..CompletionItem::default()
                    })
                    .collect(),
                ..CompletionList::default()
            };
            CompletionResponse::List(completion_list)
        }))
    }
}
