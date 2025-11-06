use crate::pipeline::models::{Job, Stage};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug)]
pub struct DependencyGraph<T> {
    nodes: Vec<T>,
    adjacency: HashMap<usize, Vec<usize>>,
}

impl<T> DependencyGraph<T> {
    pub fn new(nodes: Vec<T>) -> Self {
        Self {
            nodes,
            adjacency: HashMap::new(),
        }
    }

    pub fn add_dependency(&mut self, from: usize, to: usize) {
        self.adjacency.entry(from).or_insert_with(Vec::new).push(to);
    }

    /// Topological sort using Kahn's algorithm
    /// Returns nodes in order they can be executed
    pub fn topological_sort(&self) -> Result<Vec<usize>, String> {
        let mut in_degree: HashMap<usize, usize> = HashMap::new();
        
        // Calculate in-degrees
        for i in 0..self.nodes.len() {
            in_degree.insert(i, 0);
        }
        
        for deps in self.adjacency.values() {
            for &dep in deps {
                *in_degree.get_mut(&dep).unwrap() += 1;
            }
        }

        // Find all nodes with in-degree 0
        let mut queue: VecDeque<usize> = VecDeque::new();
        for (&node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node);
            }
        }

        let mut result = Vec::new();
        
        while let Some(node) = queue.pop_front() {
            result.push(node);
            
            if let Some(deps) = self.adjacency.get(&node) {
                for &dep in deps {
                    let degree = in_degree.get_mut(&dep).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err("Circular dependency detected".to_string());
        }

        Ok(result)
    }

    /// Get nodes that can run in parallel (same level in dependency tree)
    pub fn get_execution_levels(&self) -> Result<Vec<Vec<usize>>, String> {
        let mut in_degree: HashMap<usize, usize> = HashMap::new();
        
        // Calculate in-degrees
        for i in 0..self.nodes.len() {
            in_degree.insert(i, 0);
        }
        
        for deps in self.adjacency.values() {
            for &dep in deps {
                *in_degree.get_mut(&dep).unwrap() += 1;
            }
        }

        let mut levels = Vec::new();
        let mut processed = HashSet::new();

        while processed.len() < self.nodes.len() {
            let mut current_level = Vec::new();
            
            for i in 0..self.nodes.len() {
                if !processed.contains(&i) && in_degree[&i] == 0 {
                    current_level.push(i);
                }
            }

            if current_level.is_empty() {
                return Err("Circular dependency detected".to_string());
            }

            for &node in &current_level {
                processed.insert(node);
                if let Some(deps) = self.adjacency.get(&node) {
                    for &dep in deps {
                        *in_degree.get_mut(&dep).unwrap() -= 1;
                    }
                }
            }

            levels.push(current_level);
        }

        Ok(levels)
    }

    pub fn get_node(&self, index: usize) -> Option<&T> {
        self.nodes.get(index)
    }
}

pub fn build_stage_graph(stages: &[Stage]) -> Result<DependencyGraph<Stage>, String> {
    let mut graph = DependencyGraph::new(stages.to_vec());
    let stage_indices: HashMap<String, usize> = stages
        .iter()
        .enumerate()
        .map(|(i, s)| (s.stage.clone(), i))
        .collect();

    for (i, stage) in stages.iter().enumerate() {
        for dep in &stage.depends_on {
            if let Some(&dep_index) = stage_indices.get(dep) {
                graph.add_dependency(dep_index, i);
            } else {
                return Err(format!("Stage '{}' depends on unknown stage '{}'", stage.stage, dep));
            }
        }
    }

    Ok(graph)
}

pub fn build_job_graph(jobs: &[Job]) -> Result<DependencyGraph<Job>, String> {
    let mut graph = DependencyGraph::new(jobs.to_vec());
    let job_indices: HashMap<String, usize> = jobs
        .iter()
        .enumerate()
        .map(|(i, j)| (j.job.clone(), i))
        .collect();

    for (i, job) in jobs.iter().enumerate() {
        for dep in &job.depends_on {
            if let Some(&dep_index) = job_indices.get(dep) {
                graph.add_dependency(dep_index, i);
            } else {
                return Err(format!("Job '{}' depends on unknown job '{}'", job.job, dep));
            }
        }
    }

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dependency() {
        let mut graph = DependencyGraph::new(vec!["A", "B", "C"]);
        graph.add_dependency(0, 1); // A -> B
        graph.add_dependency(1, 2); // B -> C

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted, vec![0, 1, 2]);
    }

    #[test]
    fn test_parallel_dependencies() {
        let mut graph = DependencyGraph::new(vec!["A", "B", "C", "D"]);
        graph.add_dependency(0, 2); // A -> C
        graph.add_dependency(1, 2); // B -> C
        graph.add_dependency(2, 3); // C -> D

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0].len(), 2); // A and B can run in parallel
        assert!(levels[0].contains(&0) && levels[0].contains(&1));
    }

    #[test]
    fn test_circular_dependency() {
        let mut graph = DependencyGraph::new(vec!["A", "B", "C"]);
        graph.add_dependency(0, 1); // A -> B
        graph.add_dependency(1, 2); // B -> C
        graph.add_dependency(2, 0); // C -> A (circular!)

        assert!(graph.topological_sort().is_err());
    }
}
