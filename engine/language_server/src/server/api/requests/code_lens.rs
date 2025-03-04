use crate::server::api::traits::{RequestHandler, SyncRequestHandler};
use crate::server::api::ResultExt;
use crate::server::client::Requester;
use crate::server::{client::Notifier, Result};
use crate::Session;
use baml_schema_build::runtime_wasm::WasmSpan;
use lsp_types::{request, CodeLensParams, Command, Position, Range};

pub struct CodeLens;

impl RequestHandler for CodeLens {
    type RequestType = request::CodeLensRequest;
}

impl SyncRequestHandler for CodeLens {
    fn run(
        session: &mut Session,
        _notifier: Notifier,
        _requester: &mut Requester,
        params: CodeLensParams,
    ) -> Result<Option<Vec<lsp_types::CodeLens>>> {
        session
            .ensure_project_db_for_baml_file(&params.text_document.uri)
            .internal_error()?;
        let project = session
            .default_project_db_mut()
            .expect("Ensured that a project db exists");

        let mk_range = |span: &WasmSpan| {
            Range::new(
                Position::new(span.start_line as u32, span.start as u32),
                Position::new(span.end_line as u32, span.end as u32),
            )
        };

        let doc_matches = |span: &WasmSpan| {
            let matches =
                Some(params.text_document.uri.path()) == span.file_path.strip_prefix("file://");
            matches
        };

        let mut function_lenses: Vec<lsp_types::CodeLens> = project
            .list_functions()
            .unwrap_or(vec![])
            .iter()
            .filter(|func| doc_matches(&func.span))
            .map(|func| {
                let range = mk_range(&func.span);
                let command = Command::new(
                    "▶ Open Playground ✨".to_string(),
                    "baml.openBamlPanel".to_string(),
                    Some(vec![serde_json::json!({
                        "projectId": project.root_path(),
                        "functionName": func.name.clone(),
                        "showTests": true,
                    })]),
                );
                let lens = lsp_types::CodeLens {
                    range,
                    command: Some(command),
                    data: None,
                };
                lens
            })
            .collect();

        let test_case_lenses: Vec<lsp_types::CodeLens> = project
            .list_testcases()
            .unwrap_or(vec![])
            .iter()
            .filter(|testcase| doc_matches(&testcase.span))
            .map(|testcase| {
                let range = mk_range(&testcase.span);
                let command_name = if testcase.parent_functions.len() > 1 {
                    format!("▶ Run for ${} 💥 ", testcase.parent_functions[0].name)
                } else {
                    "▶ Run Test 💥".to_string()
                };
                let command = Command::new(
                    command_name,
                    "baml.runBamlTest".to_string(),
                    Some(vec![serde_json::json!({
                        "projectId": project.root_path(),
                        "testCaseName": testcase.name.clone(),
                        "functionName": testcase.parent_functions[0].name.clone(),
                        "showTests": true,
                    })]),
                );
                let lens = lsp_types::CodeLens {
                    range,
                    command: Some(command),
                    data: None,
                };
                lens
            })
            .collect();

        function_lenses.extend(test_case_lenses);

        Ok(Some(function_lenses))
    }
}
