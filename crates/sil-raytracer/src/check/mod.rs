//! Scene checker: reports what in a `.cfg` scene is wrong or silently ignored, with line numbers,
//! before a job is sent or a preview is drawn. The site runs the same engine (WASM build), so the
//! rules exist once; `tests/fixtures/check_cases.json` is shared with the site's test suite.

mod object;
mod report;
mod rules;
mod run;
mod walk;

pub use report::{Issue, Report, Severity, Summary};
pub use run::check_scene;
