use baml_runtime::InternalRuntimeInterface;
use log::info;
use lsp_server::ErrorCode;
use lsp_types::DiagnosticSeverity;
use lsp_types::{notification::PublishDiagnostics, PublishDiagnosticsParams, Url};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::baml_text_size::TextSize;
use crate::server::client::Notifier;
use crate::server::Result;
use crate::Session;

use super::LSPResult;

pub(super) fn clear_diagnostics(uri: &Url, notifier: &Notifier) -> Result<()> {
    notifier
        .notify::<PublishDiagnostics>(PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics: vec![],
            version: None,
        })
        .with_failure_code(ErrorCode::InternalError)?;
    Ok(())
}

// TODO: This assumes a single project. Fix.
// TODO: Handle errors.
pub fn session_lsp_diagnostics(session: &Session, file_url: &Url) -> Vec<lsp_types::Diagnostic> {
    let keys = session.index().documents.keys();
    info!("session_lsp_diagnostics. index keys: {:?}", keys);

    let (root_path, proj) = match session.projects_by_workspace_folder.iter().next() {
        Some((root_path, proj)) => (root_path, proj),
        None => {
            tracing::warn!("No project found in session");
            return vec![];
        }
    };

    dbg!(&proj.current_runtime.is_some());
    let fake_env = HashMap::new();
    info!("baml_project runtime on {:?}", proj.baml_project);
    let baml_diagnostics = match proj.baml_project.runtime(fake_env) {
        Ok(runtime) => {
            tracing::info!("OK Diagnostics: {:?}", runtime.internal().diagnostics());
            runtime.internal().diagnostics().clone()
            // Diagnostics::new(PathBuf::from("/fake1"))
        }
        Err(err) => {
            tracing::info!("Err Diagnostics: {:?}", err);
            // let mut diagnostics = internal_baml_diagnostics::Diagnostics::new(PathBuf::new());
            // diagnostics.push_error(err);
            err
            // Diagnostics::new(PathBuf::from("/fake2"))
        }
    };

    let spans = baml_diagnostics
        .errors()
        .iter()
        .map(|error| ("ERROR", error.span()))
        .chain(
            baml_diagnostics
                .warnings()
                .iter()
                .map(|warning| ("WARNING", warning.span())),
        )
        .collect::<Vec<_>>();
    info!("SPANS: {:?}", spans);

    let errors = baml_diagnostics
        .errors()
        .iter()
        .filter(|e| matches_target(root_path, file_url, &e.span()))
        .map(|error| {
            lsp_types::Diagnostic::new(
                span_to_range(session, root_path, file_url, error.span()).expect("Need a range"),
                Some(DiagnosticSeverity::ERROR),
                None,
                None,
                error.message().to_string(),
                None,
                None,
            )
        });
    let warnings = baml_diagnostics
        .warnings()
        .iter()
        .filter(|w| matches_target(root_path, file_url, &w.span()))
        .map(|warning| {
            lsp_types::Diagnostic::new(
                span_to_range(session, root_path, file_url, warning.span()).expect("Need a range"),
                Some(DiagnosticSeverity::WARNING),
                None,
                None,
                warning.message().to_string(),
                None,
                None,
            )
        });
    errors.chain(warnings).collect()
}

fn matches_target(
    project_root: &Path,
    target: &Url,
    span: &internal_baml_diagnostics::Span,
) -> bool {
    if let Some(span_path) = span.file.path().strip_prefix("file://") {
        PathBuf::from(target.path()) == ensure_absolute(project_root, &PathBuf::from(span_path))
    } else {
        tracing::warn!("Encountered a span with non-url path: {:?}", span);
        false
    }
}

/// Convert a baml Span into a lsp_types::Range for use in an `lsp_types::Diagnostic.
/// Params:
///   - session: Pass the server session, we'll need it for getting the span's
///     document's line index.
///   - project_root: Root of the baml project, needed for augmenting span paths, which
///     seem to sporadically be absolute paths.
///   - file_url: The absolute file:/// url of the file whose diagnostics we care about.
///     spans not related to this URL will be filtered out.
///   - span: The baml span to convert.
fn span_to_range(
    session: &Session,
    project_root: &Path,
    _file_url: &Url,
    span: &internal_baml_diagnostics::Span,
) -> Option<lsp_types::Range> {
    info!("SPAN_TO_RANGE({:?},{:?})", project_root, span.file.path());

    let span_path_with_prefix = span.file.path();
    let span_path = span_path_with_prefix.strip_prefix("file://")?;
    info!("span_path: {}", span_path);
    info!(
        "absolute_path = join {:?} with {:?}",
        project_root, span_path
    );
    let absolute_path = project_root.join(span_path).clone();
    dbg!(&absolute_path);

    // info!("About to URL::parse {:?}", absolute_path);
    // let url = Url::from_file_path(span_path)
    //     .or(Url::from_file_path(absolute_path))
    //     .expect("Should parse");
    let doc_key = Url::from_file_path(ensure_absolute(project_root, &PathBuf::from(span_path)))
        .expect("Should parse2");
    info!(
        "lookup {:?} from documents.keys: {:?}",
        doc_key,
        session.index.as_ref().unwrap().documents.keys()
    );
    let doc = session
        .index
        .as_ref()
        .and_then(|i| i.documents.get(&doc_key))
        .expect("Should exist");
    let line_index = doc.as_text().unwrap().index();

    let start_loc =
        line_index.source_location(TextSize::new(span.start as u32), span.file.as_str());
    let end_loc = line_index.source_location(TextSize::new(span.end as u32), span.file.as_str());

    let (start_line, start_col) = (
        start_loc.row.to_zero_indexed(),
        start_loc.column.to_zero_indexed(),
    );
    let (end_line, end_col) = (
        end_loc.row.to_zero_indexed(),
        end_loc.column.to_zero_indexed(),
    );
    Some(lsp_types::Range {
        start: lsp_types::Position::new(start_line as u32, start_col as u32),
        end: lsp_types::Position::new(end_line as u32, end_col as u32),
    })
}

/// For a project root and a path to a file in that project, return an absolute path
/// to that file.
/// This function is taylored to the quirks of spans coming from baml_runtime, which
/// sometimes include absolute paths to the source files and sometimes include
/// "relative" paths (scare-quotes are used because these paths prefixed with `/`,
/// making them technically absolute).
fn ensure_absolute(project_root: &Path, file_path: &Path) -> PathBuf {
    let file_path_relative = file_path
        .strip_prefix(std::path::MAIN_SEPARATOR_STR)
        .unwrap_or(file_path);

    if file_path
        .to_str()
        .unwrap()
        .starts_with(project_root.to_str().unwrap())
    {
        info!("No joining needed, returning {:?}", file_path);
        PathBuf::from(file_path)
    } else {
        info!(
            "Joining {:?} with {:?} to get {:?}",
            project_root,
            file_path,
            project_root.join(file_path)
        );
        project_root.join(file_path_relative)
    }
}
