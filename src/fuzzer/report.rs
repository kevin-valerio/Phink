use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct CoverageTracker {
    coverage: HashMap<String, Vec<bool>>,
    hit_lines: HashSet<usize>,
}

impl CoverageTracker {
    pub fn new(coverage_string: &str) -> Self {
        let hit_lines = coverage_string
            .split(", ")
            .filter_map(|s| s.strip_prefix("COV="))
            .filter_map(|s| s.parse().ok())
            .collect();

        CoverageTracker {
            coverage: HashMap::new(),
            hit_lines,
        }
    }

    pub fn process_file(&mut self, file_path: &str) -> std::io::Result<()> {
        let content = fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().collect();

        let mut file_coverage = vec![false; lines.len()];
        let mut block_stack = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check if we're entering a new block
            if trimmed.ends_with('{') {
                block_stack.push(i);
            }

            // Check if we're exiting a block
            if trimmed.contains('}') {
                if let Some(start) = block_stack.pop() {
                    if file_coverage[start] {
                        // If the start of the block is covered, cover everything up to this line
                        for j in start..=i {
                            file_coverage[j] = true;
                        }
                    }
                }
            }

            if let Some(cov_num) = trimmed.strip_prefix("ink::env::debug_println!(\"COV={}\", ") {
                if let Some(cov_num) = cov_num.strip_suffix(");") {
                    if let Ok(num) = cov_num.parse::<usize>() {
                        if self.hit_lines.contains(&num) {
                            // Mark the current line and previous non-empty lines as covered
                            file_coverage[i] = true;
                            for j in (0..i).rev() {
                                if !lines[j].trim().is_empty() {
                                    file_coverage[j] = true;
                                    break;
                                }
                            }

                            // Mark the start of the current block as covered
                            if let Some(&block_start) = block_stack.last() {
                                file_coverage[block_start] = true;
                            }
                        }
                    }
                }
            }
        }

        self.coverage.insert(file_path.to_string(), file_coverage);
        Ok(())
    }

    pub fn generate_report(&self, output_dir: &str) -> std::io::Result<()> {
        fs::create_dir_all(output_dir)?;

        let mut index_html = String::from(
            "<!DOCTYPE html>
                        <html>
                        <head>
                            <title>Phink Coverage Report</title>
                            <style>
                                body {
                                    font-family: Arial, sans-serif;
                                    margin: 40px;
                                    background-color: #f4f4f9;
                                }
                                h1 {
                                    color: #333;
                                }
                                ul {
                                    list-style-type: none;
                                    padding: 0;
                                }
                                li {
                                    margin: 10px 0;
                                }
                                a {
                                    text-decoration: none;
                                    color: #007bff;
                                }
                                a:hover {
                                    text-decoration: underline;
                                }
                            </style>
                        </head>
                        <body>
                            <h1>Phink Coverage Report</h1>
                            <ul>",
        );

        for (file_path, coverage) in &self.coverage {
            let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();
            let report_path = format!("{}/{}.html", output_dir.to_string(), file_name);

            self.generate_file_report(file_path, coverage, &report_path)?;

            index_html.push_str(&format!(
                "<li><a href='{}.html'>- {}</a></li>",
                file_name, file_path
            ));
        }

        index_html.push_str("</ul></body></html>");
        fs::write(format!("{}/index.html", output_dir), index_html)?;

        Ok(())
    }

    fn generate_file_report(
        &self,
        file_path: &str,
        coverage: &[bool],
        output_path: &str,
    ) -> std::io::Result<()> {
        let source_code = fs::read_to_string(file_path)?;
        let lines: Vec<&str> = source_code.lines().collect();

        let mut html = String::from(
            "<!DOCTYPE html><html><head><title>Phink File Coverage</title><style>
            .covered { background-color: #90EE90; }
            /*.uncovered { background-color: #FFB6C1; }*/
            </style></head><body>",
        );

        html.push_str(&format!("<h1>Coverage for {}</h1><pre>", file_path));

        for (i, line) in lines.iter().enumerate() {
            let line_class = if coverage[i] { "covered" } else { "uncovered" };
            if !line.contains("ink::env::debug_println!(\"COV={}\", ") {
                html.push_str(&format!(
                    "<span class='{}'>{:4} | {}</span>\n",
                    line_class,
                    i + 1,
                    html_escape(line)
                ));
            }
        }

        html.push_str("</pre></body></html>");
        fs::write(output_path, html)?;

        Ok(())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_tracking() -> std::io::Result<()> {
        let mut tracker = CoverageTracker::new("COV=236, COV=237, COV=238");
        tracker.process_file("/tmp/ink_fuzzed_Bb9Zp/lib.rs")?;
        tracker.generate_report("/tmp/ink_fuzzed_Bb9Zp/coverage_report")?;

        Ok(())
    }

    #[test]
    fn test_coverage_line_parsing() {
        let coverage_string = "COV=123, COV=125, COV=127";
        let tracker = CoverageTracker::new(coverage_string);

        let test_lines = vec![
            "    pub fn some_function() {",
            "        ink::env::debug_println!(\"COV=\", 123);",
            "        let x = 5;",
            "        ink::env::debug_println!(\"COV=\", 124);",
            "        ink::env::debug_println!(\"COV=\", 125);",
            "        if x > 3 {",
            "            ink::env::debug_println!(\"COV=\", 126);",
            "        } else {",
            "            ink::env::debug_println!(\"COV=\", 127);",
            "        }",
            "    }",
        ];

        let mut file_coverage = vec![false; test_lines.len()];

        for (i, line) in test_lines.iter().enumerate() {
            if let Some(cov_num) = line
                .trim()
                .strip_prefix("ink::env::debug_println!(\"COV=\", ")
            {
                if let Some(cov_num) = cov_num.strip_suffix(");") {
                    if let Ok(num) = cov_num.parse::<usize>() {
                        if tracker.hit_lines.contains(&num) {
                            file_coverage[i] = true;
                        }
                    }
                }
            }
        }

        assert_eq!(
            file_coverage,
            vec![
                false, // pub fn some_function() {
                true,  // ink::env::debug_println!("COV=", 123);
                false, // let x = 5;
                false, // ink::env::debug_println!("COV=", 124);
                true,  // ink::env::debug_println!("COV=", 125);
                false, // if x > 3 {
                false, // ink::env::debug_println!("COV=", 126);
                false, // } else {
                true,  // ink::env::debug_println!("COV=", 127);
                false, // }
                false, // }
            ]
        );
    }
}
