to dynamically find root nodes instead of using scene.nodes
```rust
    fn get_root_nodes(g_nodes: &Vec<NodeWrapper>) -> Vec<usize> {
        let mut children = HashSet::<usize>::new();
        for node in g_nodes {
            for i in node.child_indices.iter() {
                children.insert(*i);
            }
        }
        // root nodes are the indices from 0 to g_nodes.len() that aren't in this list
        let mut root_nodes_ids = Vec::<usize>::new();
        for i in 0..g_nodes.len() {
            if !children.contains(&i) {
                root_nodes_ids.push(i);
            }
        }
        root_nodes_ids
    }
```
