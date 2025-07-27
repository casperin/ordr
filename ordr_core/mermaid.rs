use std::collections::HashMap;

use crate::{Job, State};

/// Builds a simple mermaid diagram of the nodes that will be executed when running this job.
#[must_use]
pub fn mermaid<S: State>(job: &Job<S>) -> String {
    let idx: HashMap<_, _> = job.adj.keys().enumerate().map(|(i, id)| (*id, i)).collect();
    let n = |id| format!("n{}", idx[id]);
    let mut lines = vec!["flowchart LR".into()];

    for id in job.adj.keys() {
        lines.push(format!("{}[{}]", n(id), job.name(id)));
    }

    for (id, deps) in &job.adj {
        if deps.is_empty() {
            continue;
        }
        let deps = deps.iter().map(n).collect::<Vec<_>>().join(" & ");
        lines.push(format!("{deps} --> {}", n(id)));
    }

    lines.join("\n    ")
}
