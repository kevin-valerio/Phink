use crate::utils;
use std::fs::File;
use std::hint::black_box;
use std::io::Write;

pub type CoverageTrace = Vec<u8>;

#[derive(Clone)]
pub struct Coverage {
    branches: Vec<CoverageTrace>,
    max_coverage: u32,
}

impl Coverage {
    pub fn new(max_coverage: u32) -> Self {
        Coverage {
            branches: Vec::new(),
            max_coverage,
        }
    }

    pub fn add_cov(&mut self, coverage: &CoverageTrace) {
        self.branches.push(coverage.clone());
    }

    /// This function takes a `CoverageTrace` and remove all the coverage from the trace
    /// 'COV=153 COV=154 panicked at lib.rs:157:24: index out of bounds' =>
    /// 'panicked at lib.rs:157:24: index out of bounds'
    pub fn remove_cov_from_trace(trace: CoverageTrace) -> Vec<u8> {
        let cleaned_str = String::from_utf8_lossy(&trace)
            .split_whitespace()
            .filter(|&s| !s.starts_with("COV="))
            .collect::<Vec<&str>>()
            .join(" ");

        cleaned_str.into_bytes()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let serialized = serde_json::to_string(&self.branches).unwrap();
        let mut file = File::create("./output/phink/traces.cov")?;
        file.write_all(serialized.as_bytes())?;

        Ok(())
    }

    /// This function create an artificial coverage to convince ziggy that a message is interesting or not.
    pub fn redirect_coverage(&self) {
        let flatten_cov: Vec<u8> = self.branches.clone().into_iter().flatten().collect();
        let coverage_str = utils::deduplicate(&String::from_utf8_lossy(&flatten_cov));
        let coverage_lines: Vec<&str> = coverage_str.split('\n').collect();

        println!("[🚧DEBUG TRACE] : {:?}", coverage_lines);
        // println!("[🚧MAX REACHABLE COVERAGE] : {:?}", &self.max_coverage);
        seq_macro::seq!(x in 0..=500 {
            let target = format!("COV={}", x);
            if coverage_lines.contains(&target.as_str()) {
                let _ = black_box(x + 1);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_cov_from_trace_simple() {
        let input = b"COV=153 panicked at ..x/lib.rs:157:24: index out of bounds".to_vec();
        let expected_output = b"panicked at ..x/lib.rs:157:24: index out of bounds".to_vec();
        assert_eq!(Coverage::remove_cov_from_trace(input), expected_output);
    }

    #[test]
    fn test_remove_cov_from_trace_multiple() {
        let input = b"COV=153 COV=154 panicked at ..x/lib.rs:157:24: index out of bounds".to_vec();
        let expected_output = b"panicked at ..x/lib.rs:157:24: index out of bounds".to_vec();
        assert_eq!(Coverage::remove_cov_from_trace(input), expected_output);
    }

    #[test]
    fn test_remove_cov_from_trace_with_other_text() {
        let input =
            b"error COV=153 occurred at ..x/lib.rs:157:24: COV=154 index out of bounds".to_vec();
        let expected_output = b"error occurred at ..x/lib.rs:157:24: index out of bounds".to_vec();
        assert_eq!(Coverage::remove_cov_from_trace(input), expected_output);
    }

    #[test]
    fn test_remove_cov_from_trace_no_cov() {
        let input = b"panicked at ..x/lib.rs:157:24: index out of bounds".to_vec();
        let expected_output = b"panicked at ..x/lib.rs:157:24: index out of bounds".to_vec();
        assert_eq!(Coverage::remove_cov_from_trace(input), expected_output);
    }

    #[test]
    fn test_remove_cov_from_trace_empty() {
        let input = b"".to_vec();
        let expected_output = b"".to_vec();
        assert_eq!(Coverage::remove_cov_from_trace(input), expected_output);
    }

    #[test]
    fn test_remove_cov_from_trace_mixed() {
        let input =
            b"panicked COV=12345 at COV=6789 ..x/lib.rs:157:24: index out of COV=98765 bounds"
                .to_vec();
        let expected_output = b"panicked at ..x/lib.rs:157:24: index out of bounds".to_vec();
        assert_eq!(Coverage::remove_cov_from_trace(input), expected_output);
    }
}
