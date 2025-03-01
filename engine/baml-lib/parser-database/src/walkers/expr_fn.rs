use internal_baml_schema_ast::ast;

use super::Walker;

/// Walker for top level assignments.
pub type TopLevelAssignmentWalker<'db> = Walker<'db, ast::TopLevelAssignmentId>;

/// Walker for expression functions.
pub type ExprFnWalker<'db> = Walker<'db, ast::ExprFnId>;
