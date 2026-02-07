// Test Reporter
// Generates test output in JUnit XML, TAP, and terminal formats

use crate::testing::runner::TestSuiteResult;

use std::fmt;

/// Output format for test reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// JUnit XML format (for CI systems)
    JUnit,
    /// TAP (Test Anything Protocol) format
    Tap,
    /// Human-readable terminal output
    Terminal,
}

impl fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportFormat::JUnit => write!(f, "junit"),
            ReportFormat::Tap => write!(f, "tap"),
            ReportFormat::Terminal => write!(f, "terminal"),
        }
    }
}

impl std::str::FromStr for ReportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "junit" | "junit-xml" | "xml" => Ok(ReportFormat::JUnit),
            "tap" => Ok(ReportFormat::Tap),
            "terminal" | "text" | "console" => Ok(ReportFormat::Terminal),
            _ => Err(format!(
                "Unknown report format '{}'. Valid formats: junit, tap, terminal",
                s
            )),
        }
    }
}

/// Test reporter that generates output in various formats
pub struct TestReporter;

impl TestReporter {
    /// Generate a report in the specified format
    pub fn report(results: &TestSuiteResult, format: ReportFormat) -> String {
        match format {
            ReportFormat::JUnit => Self::to_junit_xml(results),
            ReportFormat::Tap => Self::to_tap(results),
            ReportFormat::Terminal => Self::to_terminal(results),
        }
    }

    /// Generate JUnit XML output
    ///
    /// Compatible with CI systems like Azure DevOps, Jenkins, GitHub Actions, etc.
    pub fn to_junit_xml(results: &TestSuiteResult) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

        // Test suite element
        xml.push_str(&format!(
            "<testsuites tests=\"{}\" failures=\"{}\" errors=\"0\" time=\"{:.3}\">\n",
            results.total,
            results.failed,
            results.duration.as_secs_f64()
        ));

        xml.push_str(&format!(
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"0\" time=\"{:.3}\">\n",
            xml_escape(&results.suite_name),
            results.total,
            results.failed,
            results.duration.as_secs_f64()
        ));

        for test in &results.results {
            xml.push_str(&format!(
                "    <testcase name=\"{}\" time=\"{:.3}\"",
                xml_escape(&test.name),
                test.duration.as_secs_f64()
            ));

            if test.passed {
                xml.push_str(" />\n");
            } else {
                xml.push_str(">\n");

                // Add failure details
                if let Some(ref msg) = test.failure_message {
                    xml.push_str(&format!(
                        "      <failure message=\"{}\">\n",
                        xml_escape(msg)
                    ));
                } else {
                    xml.push_str("      <failure message=\"Test failed\">\n");
                }

                // Add assertion details
                for assertion in &test.assertions {
                    if !assertion.passed {
                        xml.push_str(&format!(
                            "        FAIL: {}\n",
                            xml_escape(&assertion.message)
                        ));
                        if let Some(ref detail) = assertion.failure_detail {
                            xml.push_str(&format!("          {}\n", xml_escape(detail)));
                        }
                    }
                }

                xml.push_str("      </failure>\n");
                xml.push_str("    </testcase>\n");
            }
        }

        xml.push_str("  </testsuite>\n");
        xml.push_str("</testsuites>\n");
        xml
    }

    /// Generate TAP (Test Anything Protocol) output
    ///
    /// TAP version 13 compatible
    pub fn to_tap(results: &TestSuiteResult) -> String {
        let mut tap = String::new();
        tap.push_str("TAP version 13\n");
        tap.push_str(&format!("1..{}\n", results.total));

        for (i, test) in results.results.iter().enumerate() {
            let test_num = i + 1;

            if test.passed {
                tap.push_str(&format!("ok {} - {}\n", test_num, test.name));
            } else {
                tap.push_str(&format!("not ok {} - {}\n", test_num, test.name));

                // Add YAML diagnostics block
                tap.push_str("  ---\n");
                tap.push_str(&format!("  duration_ms: {}\n", test.duration.as_millis()));

                if let Some(ref msg) = test.failure_message {
                    tap.push_str(&format!("  message: \"{}\"\n", msg));
                }

                // List failed assertions
                let failed: Vec<_> = test.assertions.iter().filter(|a| !a.passed).collect();
                if !failed.is_empty() {
                    tap.push_str("  failures:\n");
                    for assertion in failed {
                        tap.push_str(&format!("    - assertion: \"{}\"\n", assertion.assertion));
                        tap.push_str(&format!("      message: \"{}\"\n", assertion.message));
                        if let Some(ref detail) = assertion.failure_detail {
                            tap.push_str(&format!("      detail: \"{}\"\n", detail));
                        }
                    }
                }

                tap.push_str("  ...\n");
            }
        }

        // Summary comment
        tap.push_str(&format!(
            "# tests {}\n# pass {}\n# fail {}\n# duration {:.3}s\n",
            results.total,
            results.passed,
            results.failed,
            results.duration.as_secs_f64()
        ));

        tap
    }

    /// Generate human-readable terminal output
    pub fn to_terminal(results: &TestSuiteResult) -> String {
        let mut out = String::new();

        // Header
        out.push_str(&format!("\nTest Suite: {}\n", results.suite_name));
        out.push_str(&"=".repeat(60));
        out.push('\n');

        // Individual test results
        for test in &results.results {
            let status = if test.passed { "PASS" } else { "FAIL" };
            let symbol = if test.passed { "+" } else { "x" };

            out.push_str(&format!(
                "  [{}] {} ({:.2}s) {}\n",
                symbol,
                status,
                test.duration.as_secs_f64(),
                test.name,
            ));

            // Show assertion details for failures
            if !test.passed {
                for assertion in &test.assertions {
                    if !assertion.passed {
                        out.push_str(&format!("       FAIL: {}\n", assertion.message));
                        if let Some(ref detail) = assertion.failure_detail {
                            out.push_str(&format!("             {}\n", detail));
                        }
                    }
                }
            }
        }

        // Summary
        out.push_str(&"-".repeat(60));
        out.push('\n');

        let status_line = if results.failed == 0 {
            format!(
                "  All {} tests passed ({:.2}s)",
                results.total,
                results.duration.as_secs_f64()
            )
        } else {
            format!(
                "  {} of {} tests failed ({:.2}s)",
                results.failed,
                results.total,
                results.duration.as_secs_f64()
            )
        };
        out.push_str(&status_line);
        out.push('\n');

        if results.skipped > 0 {
            out.push_str(&format!("  {} tests skipped\n", results.skipped));
        }

        out.push('\n');
        out
    }
}

/// Escape special XML characters
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assertions::AssertionResult;
    use crate::testing::runner::TestResult;
    use std::time::Duration;

    fn make_passing_test(name: &str) -> TestResult {
        TestResult {
            name: name.to_string(),
            passed: true,
            duration: Duration::from_millis(150),
            assertions: vec![AssertionResult {
                assertion: "pipeline_succeeded".to_string(),
                passed: true,
                message: "Pipeline completed successfully".to_string(),
                failure_detail: None,
            }],
            failure_message: None,
            pipeline_path: "pipeline.yml".to_string(),
        }
    }

    fn make_failing_test(name: &str) -> TestResult {
        TestResult {
            name: name.to_string(),
            passed: false,
            duration: Duration::from_millis(300),
            assertions: vec![
                AssertionResult {
                    assertion: "step_succeeded(Build)".to_string(),
                    passed: true,
                    message: "Step 'Build' has status Succeeded".to_string(),
                    failure_detail: None,
                },
                AssertionResult {
                    assertion: "step_succeeded(Deploy)".to_string(),
                    passed: false,
                    message: "Step 'Deploy' expected Succeeded but was Failed".to_string(),
                    failure_detail: Some(
                        "Actual status: Failed, error: connection timeout".to_string(),
                    ),
                },
            ],
            failure_message: Some("1 of 2 assertions failed".to_string()),
            pipeline_path: "pipeline.yml".to_string(),
        }
    }

    fn make_suite_result() -> TestSuiteResult {
        TestSuiteResult {
            suite_name: "Integration Tests".to_string(),
            results: vec![
                make_passing_test("Build succeeds"),
                make_failing_test("Deploy works"),
                make_passing_test("Cleanup runs"),
            ],
            total: 3,
            passed: 2,
            failed: 1,
            skipped: 0,
            duration: Duration::from_secs(2),
        }
    }

    #[test]
    fn test_junit_xml_output() {
        let results = make_suite_result();
        let xml = TestReporter::to_junit_xml(&results);

        assert!(xml.starts_with("<?xml version=\"1.0\""));
        assert!(xml.contains("<testsuites"));
        assert!(xml.contains("tests=\"3\""));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("name=\"Build succeeds\""));
        assert!(xml.contains("name=\"Deploy works\""));
        assert!(xml.contains("<failure"));
        assert!(xml.contains("connection timeout"));
    }

    #[test]
    fn test_tap_output() {
        let results = make_suite_result();
        let tap = TestReporter::to_tap(&results);

        assert!(tap.starts_with("TAP version 13\n"));
        assert!(tap.contains("1..3\n"));
        assert!(tap.contains("ok 1 - Build succeeds"));
        assert!(tap.contains("not ok 2 - Deploy works"));
        assert!(tap.contains("ok 3 - Cleanup runs"));
        assert!(tap.contains("# tests 3"));
        assert!(tap.contains("# pass 2"));
        assert!(tap.contains("# fail 1"));
    }

    #[test]
    fn test_terminal_output() {
        let results = make_suite_result();
        let terminal = TestReporter::to_terminal(&results);

        assert!(terminal.contains("Test Suite: Integration Tests"));
        assert!(terminal.contains("[+] PASS"));
        assert!(terminal.contains("[x] FAIL"));
        assert!(terminal.contains("Build succeeds"));
        assert!(terminal.contains("Deploy works"));
        assert!(terminal.contains("1 of 3 tests failed"));
    }

    #[test]
    fn test_terminal_all_pass() {
        let results = TestSuiteResult {
            suite_name: "All Pass".to_string(),
            results: vec![make_passing_test("Test 1"), make_passing_test("Test 2")],
            total: 2,
            passed: 2,
            failed: 0,
            skipped: 0,
            duration: Duration::from_millis(500),
        };
        let terminal = TestReporter::to_terminal(&results);
        assert!(terminal.contains("All 2 tests passed"));
    }

    #[test]
    fn test_xml_escaping() {
        assert_eq!(xml_escape("<test>"), "&lt;test&gt;");
        assert_eq!(xml_escape("a & b"), "a &amp; b");
        assert_eq!(xml_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_report_format_parsing() {
        assert_eq!(
            "junit".parse::<ReportFormat>().unwrap(),
            ReportFormat::JUnit
        );
        assert_eq!("xml".parse::<ReportFormat>().unwrap(), ReportFormat::JUnit);
        assert_eq!("tap".parse::<ReportFormat>().unwrap(), ReportFormat::Tap);
        assert_eq!(
            "terminal".parse::<ReportFormat>().unwrap(),
            ReportFormat::Terminal
        );
        assert!("unknown".parse::<ReportFormat>().is_err());
    }

    #[test]
    fn test_report_dispatches_correctly() {
        let results = make_suite_result();

        let junit = TestReporter::report(&results, ReportFormat::JUnit);
        assert!(junit.contains("<?xml"));

        let tap = TestReporter::report(&results, ReportFormat::Tap);
        assert!(tap.contains("TAP version"));

        let terminal = TestReporter::report(&results, ReportFormat::Terminal);
        assert!(terminal.contains("Test Suite:"));
    }
}
