use crate::{
    error::Error,
    graph::{Ctx, Er},
    job::Job,
    node::Node,
};

/// Validates a graph.
pub(crate) fn validate_nodes<C: Ctx, E: Er>(
    nodes: &[Node<C, E>],
) -> Result<Vec<Vec<usize>>, Error<E>> {
    if nodes.is_empty() {
        return Err(Error::NoNodes);
    }

    // Build adj
    let mut adj = Vec::with_capacity(nodes.len());
    for node in nodes {
        let mut deps = vec![];
        for dep in &node.deps {
            let index = nodes
                .binary_search_by_key(dep, |node| node.id)
                .map_err(|_| Error::DependencyNotFound(node.name, *dep))?;

            deps.push(index);
        }
        adj.push(deps);
    }

    if let Some(cycle) = find_cycle(&adj) {
        let names: Vec<_> = cycle.iter().map(|&i| nodes[i].name).collect();
        return Err(Error::Cycle(names));
    }

    Ok(adj)
}

/// Searches a set of nodes (in the graph) for any cycles.
///
/// We are operating with DAG's, so it's important that they are devoid of cycles.
///
/// # Panics
/// If we fucked up.
#[must_use]
pub fn find_cycle(adj: &[Vec<usize>]) -> Option<Vec<usize>> {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        /// We haven't yet looked at this node.
        New,
        /// Currently looking at this node (or its children).
        Active,
        /// Done and found no errors.
        Done,
    }

    let len = adj.len();
    let mut state = vec![State::New; len];
    let mut parents = vec![usize::MAX; len]; // MAX = no known parent
    let mut stack = vec![];

    // Loop through all nodes as a potential starting point.
    for i in 0..len {
        // If we have already checked it out, then disregard it.
        if state[i] != State::New {
            continue;
        }

        // Push our starting point onto the stack.
        stack.push(i);

        while let Some(&i) = stack.last() {
            if state[i] == State::New {
                state[i] = State::Active;
            } else {
                state[i] = State::Done;
                stack.pop();
            }

            for &v in &adj[i] {
                parents[v] = i;
                let node_state = state[v];
                match node_state {
                    State::New => stack.push(v),

                    State::Active => {
                        // Cycle found. Build the path and return that.
                        let mut parent = parents[v];
                        let mut path = vec![v, parent];

                        while parent != v {
                            parent = parents[parent];
                            path.push(parent);
                        }

                        return Some(path);
                    }

                    State::Done => {} // noop
                }
            }
        }
    }

    None
}

/// Ensures that targets and inputs in the job actually exist in the nodes.
pub(crate) fn validate_job<C: Ctx, E: Er>(
    nodes: &[Node<C, E>],
    job: &Job<C, E>,
) -> Result<(), Error<E>> {
    for id in job.inputs.keys() {
        if nodes.binary_search_by_key(id, |node| node.id).is_err() {
            return Err(Error::NodeNotFound(*id));
        }
    }

    for id in &job.targets {
        if nodes.binary_search_by_key(id, |node| node.id).is_err() {
            return Err(Error::NodeNotFound(*id));
        }
    }

    Ok(())
}
