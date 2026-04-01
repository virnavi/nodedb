pub mod error;
pub mod types;
pub mod engine;
pub mod traversal;
pub mod algorithm;

pub use error::GraphError;
pub use types::{GraphNode, GraphEdge, DeleteBehaviour, TraversalResult};
pub use engine::GraphEngine;
pub use traversal::{bfs, dfs, shortest_path, multi_hop};
pub use algorithm::{pagerank, connected_components, has_cycle, find_cycles};
