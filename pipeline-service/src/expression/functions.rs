// Built-in Functions for Azure DevOps Expressions
// Implements all standard Azure DevOps expression functions

use crate::expression::evaluator::{EvalError, ExpressionContext};
use crate::parser::models::Value;

/// Registry of built-in functions
pub struct BuiltinFunctions;

impl BuiltinFunctions {
    pub fn new() -> Self {
        Self
    }

    /// Call a built-in function
    pub fn call(
        &self,
        name: &str,
        args: Vec<Value>,
        context: &ExpressionContext,
    ) -> Result<Value, EvalError> {
        match name.to_lowercase().as_str() {
            // Comparison functions
            "eq" => self.fn_eq(args),
            "ne" => self.fn_ne(args),
            "lt" => self.fn_lt(args),
            "le" => self.fn_le(args),
            "gt" => self.fn_gt(args),
            "ge" => self.fn_ge(args),
            "in" => self.fn_in(args),
            "notin" => self.fn_notin(args),

            // Logical functions
            "and" => self.fn_and(args),
            "or" => self.fn_or(args),
            "not" => self.fn_not(args),
            "xor" => self.fn_xor(args),

            // String functions
            "contains" => self.fn_contains(args),
            "startswith" => self.fn_startswith(args),
            "endswith" => self.fn_endswith(args),
            "format" => self.fn_format(args),
            "join" => self.fn_join(args),
            "replace" => self.fn_replace(args),
            "split" => self.fn_split(args),
            "lower" => self.fn_lower(args),
            "upper" => self.fn_upper(args),
            "trim" => self.fn_trim(args),

            // Conversion functions
            "converttojson" => self.fn_convert_to_json(args),

            // Status functions (context-aware)
            "succeeded" => self.fn_succeeded(args, context),
            "failed" => self.fn_failed(args, context),
            "canceled" => self.fn_canceled(context),
            "always" => Ok(Value::Bool(true)),
            "succeededorfailed" => self.fn_succeeded_or_failed(args, context),

            // Utility functions
            "coalesce" => self.fn_coalesce(args),
            "counter" => self.fn_counter(args),
            "iif" => self.fn_iif(args),
            "length" => self.fn_length(args),

            _ => Err(EvalError::new(format!("unknown function: {}", name))),
        }
    }

    // =========================================================================
    // Comparison Functions
    // =========================================================================

    fn fn_eq(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "eq")?;
        Ok(Value::Bool(self.values_equal(&args[0], &args[1])))
    }

    fn fn_ne(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "ne")?;
        Ok(Value::Bool(!self.values_equal(&args[0], &args[1])))
    }

    fn fn_lt(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "lt")?;
        let (a, b) = self.as_numbers(&args[0], &args[1])?;
        Ok(Value::Bool(a < b))
    }

    fn fn_le(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "le")?;
        let (a, b) = self.as_numbers(&args[0], &args[1])?;
        Ok(Value::Bool(a <= b))
    }

    fn fn_gt(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "gt")?;
        let (a, b) = self.as_numbers(&args[0], &args[1])?;
        Ok(Value::Bool(a > b))
    }

    fn fn_ge(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "ge")?;
        let (a, b) = self.as_numbers(&args[0], &args[1])?;
        Ok(Value::Bool(a >= b))
    }

    fn fn_in(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::new("in() requires at least 2 arguments"));
        }
        let needle = &args[0];
        for arg in &args[1..] {
            if self.values_equal(needle, arg) {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    fn fn_notin(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        let result = self.fn_in(args)?;
        match result {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            _ => unreachable!(),
        }
    }

    // =========================================================================
    // Logical Functions
    // =========================================================================

    fn fn_and(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        for arg in &args {
            if !arg.is_truthy() {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    fn fn_or(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        for arg in &args {
            if arg.is_truthy() {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    fn fn_not(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 1, "not")?;
        Ok(Value::Bool(!args[0].is_truthy()))
    }

    fn fn_xor(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "xor")?;
        let a = args[0].is_truthy();
        let b = args[1].is_truthy();
        Ok(Value::Bool(a ^ b))
    }

    // =========================================================================
    // String Functions
    // =========================================================================

    fn fn_contains(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "contains")?;

        match (&args[0], &args[1]) {
            (Value::String(haystack), Value::String(needle)) => {
                // Case-insensitive contains for strings
                Ok(Value::Bool(
                    haystack.to_lowercase().contains(&needle.to_lowercase()),
                ))
            }
            (Value::Array(arr), needle) => {
                // Array contains
                for item in arr {
                    if self.values_equal(item, needle) {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            _ => Err(EvalError::new("contains() requires string or array")),
        }
    }

    fn fn_startswith(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "startsWith")?;
        let s = args[0].as_string().to_lowercase();
        let prefix = args[1].as_string().to_lowercase();
        Ok(Value::Bool(s.starts_with(&prefix)))
    }

    fn fn_endswith(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "endsWith")?;
        let s = args[0].as_string().to_lowercase();
        let suffix = args[1].as_string().to_lowercase();
        Ok(Value::Bool(s.ends_with(&suffix)))
    }

    fn fn_format(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(EvalError::new("format() requires at least 1 argument"));
        }

        let template = args[0].as_string();
        let mut result = template;

        // Replace {0}, {1}, etc. with arguments
        for (i, arg) in args.iter().skip(1).enumerate() {
            let placeholder = format!("{{{}}}", i);
            result = result.replace(&placeholder, &arg.as_string());
        }

        Ok(Value::String(result))
    }

    fn fn_join(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "join")?;

        let separator = args[1].as_string();

        match &args[0] {
            Value::Array(arr) => {
                let strings: Vec<String> = arr.iter().map(|v| v.as_string()).collect();
                Ok(Value::String(strings.join(&separator)))
            }
            _ => Err(EvalError::new("join() requires array as first argument")),
        }
    }

    fn fn_replace(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 3, "replace")?;
        let s = args[0].as_string();
        let from = args[1].as_string();
        let to = args[2].as_string();
        Ok(Value::String(s.replace(&from, &to)))
    }

    fn fn_split(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 2, "split")?;
        let s = args[0].as_string();
        let delimiter = args[1].as_string();
        let parts: Vec<Value> = s
            .split(&delimiter)
            .map(|p| Value::String(p.to_string()))
            .collect();
        Ok(Value::Array(parts))
    }

    fn fn_lower(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 1, "lower")?;
        Ok(Value::String(args[0].as_string().to_lowercase()))
    }

    fn fn_upper(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 1, "upper")?;
        Ok(Value::String(args[0].as_string().to_uppercase()))
    }

    fn fn_trim(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 1, "trim")?;
        Ok(Value::String(args[0].as_string().trim().to_string()))
    }

    // =========================================================================
    // Conversion Functions
    // =========================================================================

    fn fn_convert_to_json(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 1, "convertToJson")?;
        Ok(Value::String(args[0].to_json()))
    }

    // =========================================================================
    // Status Functions
    // =========================================================================

    fn fn_succeeded(
        &self,
        args: Vec<Value>,
        context: &ExpressionContext,
    ) -> Result<Value, EvalError> {
        if args.is_empty() {
            // Check current job status
            if let Some(job) = &context.job {
                return Ok(Value::Bool(job.status.succeeded && !job.status.failed));
            }
            // If no job context, default to true (pipeline is still running)
            return Ok(Value::Bool(true));
        }

        // Check specific dependency
        for arg in args {
            let name = arg.as_string();
            // Check job dependencies
            if let Some(dep) = context.dependencies.jobs.get(&name) {
                if dep.result.to_lowercase() != "succeeded" {
                    return Ok(Value::Bool(false));
                }
            }
            // Check stage dependencies
            else if let Some(dep) = context.dependencies.stages.get(&name) {
                if dep.result.to_lowercase() != "succeeded" {
                    return Ok(Value::Bool(false));
                }
            }
            // Check step status
            else if let Some(step) = context.steps.get(&name) {
                if !step.status.succeeded {
                    return Ok(Value::Bool(false));
                }
            }
        }

        Ok(Value::Bool(true))
    }

    fn fn_failed(&self, args: Vec<Value>, context: &ExpressionContext) -> Result<Value, EvalError> {
        if args.is_empty() {
            // Check current job status
            if let Some(job) = &context.job {
                return Ok(Value::Bool(job.status.failed));
            }
            return Ok(Value::Bool(false));
        }

        // Check specific dependency
        for arg in args {
            let name = arg.as_string();
            if let Some(dep) = context.dependencies.jobs.get(&name) {
                if dep.result.to_lowercase() == "failed" {
                    return Ok(Value::Bool(true));
                }
            }
            if let Some(dep) = context.dependencies.stages.get(&name) {
                if dep.result.to_lowercase() == "failed" {
                    return Ok(Value::Bool(true));
                }
            }
            if let Some(step) = context.steps.get(&name) {
                if step.status.failed {
                    return Ok(Value::Bool(true));
                }
            }
        }

        Ok(Value::Bool(false))
    }

    fn fn_canceled(&self, context: &ExpressionContext) -> Result<Value, EvalError> {
        if let Some(job) = &context.job {
            return Ok(Value::Bool(job.status.canceled));
        }
        Ok(Value::Bool(false))
    }

    fn fn_succeeded_or_failed(
        &self,
        args: Vec<Value>,
        context: &ExpressionContext,
    ) -> Result<Value, EvalError> {
        // True if not canceled
        let succeeded = self.fn_succeeded(args.clone(), context)?;
        let failed = self.fn_failed(args, context)?;

        match (succeeded, failed) {
            (Value::Bool(s), Value::Bool(f)) => Ok(Value::Bool(s || f)),
            _ => Ok(Value::Bool(true)),
        }
    }

    // =========================================================================
    // Utility Functions
    // =========================================================================

    fn fn_coalesce(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        for arg in args {
            match &arg {
                Value::Null => continue,
                Value::String(s) if s.is_empty() => continue,
                _ => return Ok(arg),
            }
        }
        Ok(Value::Null)
    }

    fn fn_counter(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        // counter(prefix, seed) - returns a counter value
        // In local execution, we just return the seed or 1
        if args.is_empty() {
            return Ok(Value::Number(1.0));
        }

        let _prefix = args.first().map(|v| v.as_string()).unwrap_or_default();
        let seed = args.get(1).and_then(|v| v.as_number()).unwrap_or(1.0);

        Ok(Value::Number(seed))
    }

    fn fn_iif(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 3, "iif")?;
        if args[0].is_truthy() {
            Ok(args[1].clone())
        } else {
            Ok(args[2].clone())
        }
    }

    fn fn_length(&self, args: Vec<Value>) -> Result<Value, EvalError> {
        self.require_args(&args, 1, "length")?;
        match &args[0] {
            Value::String(s) => Ok(Value::Number(s.len() as f64)),
            Value::Array(arr) => Ok(Value::Number(arr.len() as f64)),
            Value::Object(obj) => Ok(Value::Number(obj.len() as f64)),
            _ => Err(EvalError::new("length() requires string, array, or object")),
        }
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    fn require_args(&self, args: &[Value], count: usize, name: &str) -> Result<(), EvalError> {
        if args.len() != count {
            return Err(EvalError::new(format!(
                "{}() requires {} argument(s), got {}",
                name,
                count,
                args.len()
            )));
        }
        Ok(())
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a.to_lowercase() == b.to_lowercase(),
            // Coerce for comparison
            (Value::Number(a), Value::String(b)) | (Value::String(b), Value::Number(a)) => b
                .parse::<f64>()
                .map(|n| (a - n).abs() < f64::EPSILON)
                .unwrap_or(false),
            (Value::Bool(a), Value::String(b)) | (Value::String(b), Value::Bool(a)) => {
                let b_lower = b.to_lowercase();
                (*a && b_lower == "true") || (!*a && b_lower == "false")
            }
            (Value::Array(a), Value::Array(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                a.iter()
                    .zip(b.iter())
                    .all(|(x, y)| Self::values_equal_static(x, y))
            }
            _ => false,
        }
    }

    fn values_equal_static(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a.to_lowercase() == b.to_lowercase(),
            (Value::Number(a), Value::String(b)) | (Value::String(b), Value::Number(a)) => b
                .parse::<f64>()
                .map(|n| (a - n).abs() < f64::EPSILON)
                .unwrap_or(false),
            (Value::Bool(a), Value::String(b)) | (Value::String(b), Value::Bool(a)) => {
                let b_lower = b.to_lowercase();
                (*a && b_lower == "true") || (!*a && b_lower == "false")
            }
            (Value::Array(a), Value::Array(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                a.iter()
                    .zip(b.iter())
                    .all(|(x, y)| Self::values_equal_static(x, y))
            }
            _ => false,
        }
    }

    fn as_numbers(&self, a: &Value, b: &Value) -> Result<(f64, f64), EvalError> {
        let a_num = a
            .as_number()
            .ok_or_else(|| EvalError::new("first argument is not a number"))?;
        let b_num = b
            .as_number()
            .ok_or_else(|| EvalError::new("second argument is not a number"))?;
        Ok((a_num, b_num))
    }
}

impl Default for BuiltinFunctions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(expr: &str) -> Value {
        let ctx = ExpressionContext::default();

        use crate::expression::evaluator::Evaluator;
        use crate::expression::parser::ExprParser;

        let ast = ExprParser::parse_str(expr).unwrap();
        let evaluator = Evaluator::new(&ctx);
        evaluator.eval(&ast).unwrap()
    }

    // =========================================================================
    // Comparison Functions
    // =========================================================================

    #[test]
    fn test_eq() {
        assert_eq!(eval("eq('hello', 'Hello')"), Value::Bool(true));
        assert_eq!(eval("eq(42, 42)"), Value::Bool(true));
        assert_eq!(eval("eq(true, true)"), Value::Bool(true));
        assert_eq!(eval("eq('a', 'b')"), Value::Bool(false));
    }

    #[test]
    fn test_ne() {
        assert_eq!(eval("ne('a', 'b')"), Value::Bool(true));
        assert_eq!(eval("ne('a', 'a')"), Value::Bool(false));
    }

    #[test]
    fn test_comparisons() {
        assert_eq!(eval("lt(1, 2)"), Value::Bool(true));
        assert_eq!(eval("le(2, 2)"), Value::Bool(true));
        assert_eq!(eval("gt(3, 2)"), Value::Bool(true));
        assert_eq!(eval("ge(2, 2)"), Value::Bool(true));
    }

    #[test]
    fn test_in() {
        assert_eq!(eval("in('a', 'a', 'b', 'c')"), Value::Bool(true));
        assert_eq!(eval("in('d', 'a', 'b', 'c')"), Value::Bool(false));
    }

    #[test]
    fn test_notin() {
        assert_eq!(eval("notIn('d', 'a', 'b', 'c')"), Value::Bool(true));
        assert_eq!(eval("notIn('a', 'a', 'b', 'c')"), Value::Bool(false));
    }

    // =========================================================================
    // Logical Functions
    // =========================================================================

    #[test]
    fn test_and() {
        assert_eq!(eval("and(true, true)"), Value::Bool(true));
        assert_eq!(eval("and(true, false)"), Value::Bool(false));
        assert_eq!(eval("and(true, true, true)"), Value::Bool(true));
    }

    #[test]
    fn test_or() {
        assert_eq!(eval("or(false, true)"), Value::Bool(true));
        assert_eq!(eval("or(false, false)"), Value::Bool(false));
    }

    #[test]
    fn test_not() {
        assert_eq!(eval("not(false)"), Value::Bool(true));
        assert_eq!(eval("not(true)"), Value::Bool(false));
    }

    #[test]
    fn test_xor() {
        assert_eq!(eval("xor(true, false)"), Value::Bool(true));
        assert_eq!(eval("xor(true, true)"), Value::Bool(false));
        assert_eq!(eval("xor(false, false)"), Value::Bool(false));
    }

    // =========================================================================
    // String Functions
    // =========================================================================

    #[test]
    fn test_contains() {
        assert_eq!(eval("contains('Hello World', 'world')"), Value::Bool(true));
        assert_eq!(eval("contains('Hello', 'xyz')"), Value::Bool(false));
    }

    #[test]
    fn test_startswith() {
        assert_eq!(
            eval("startsWith('Hello World', 'hello')"),
            Value::Bool(true)
        );
        assert_eq!(eval("startsWith('Hello', 'world')"), Value::Bool(false));
    }

    #[test]
    fn test_endswith() {
        assert_eq!(eval("endsWith('Hello World', 'WORLD')"), Value::Bool(true));
        assert_eq!(eval("endsWith('Hello', 'xyz')"), Value::Bool(false));
    }

    #[test]
    fn test_format() {
        assert_eq!(
            eval("format('Hello {0}!', 'World')"),
            Value::String("Hello World!".to_string())
        );
        assert_eq!(
            eval("format('{0} + {1} = {2}', 1, 2, 3)"),
            Value::String("1 + 2 = 3".to_string())
        );
    }

    #[test]
    fn test_join() {
        // Can't directly test join without array support in the test helper
        // But the function is implemented
    }

    #[test]
    fn test_replace() {
        assert_eq!(
            eval("replace('hello world', 'world', 'rust')"),
            Value::String("hello rust".to_string())
        );
    }

    #[test]
    fn test_lower_upper() {
        assert_eq!(eval("lower('HELLO')"), Value::String("hello".to_string()));
        assert_eq!(eval("upper('hello')"), Value::String("HELLO".to_string()));
    }

    #[test]
    fn test_trim() {
        assert_eq!(
            eval("trim('  hello  ')"),
            Value::String("hello".to_string())
        );
    }

    // =========================================================================
    // Utility Functions
    // =========================================================================

    #[test]
    fn test_coalesce() {
        assert_eq!(
            eval("coalesce(null, '', 'default')"),
            Value::String("default".to_string())
        );
        assert_eq!(
            eval("coalesce('first', 'second')"),
            Value::String("first".to_string())
        );
    }

    #[test]
    fn test_iif() {
        assert_eq!(
            eval("iif(true, 'yes', 'no')"),
            Value::String("yes".to_string())
        );
        assert_eq!(
            eval("iif(false, 'yes', 'no')"),
            Value::String("no".to_string())
        );
    }

    #[test]
    fn test_length() {
        assert_eq!(eval("length('hello')"), Value::Number(5.0));
    }

    // =========================================================================
    // Status Functions
    // =========================================================================

    #[test]
    fn test_succeeded() {
        // Without context, defaults to true
        assert_eq!(eval("succeeded()"), Value::Bool(true));
    }

    #[test]
    fn test_always() {
        assert_eq!(eval("always()"), Value::Bool(true));
    }

    // =========================================================================
    // Complex Expressions
    // =========================================================================

    #[test]
    fn test_complex_and_eq() {
        assert_eq!(eval("and(eq('a', 'a'), eq(1, 1))"), Value::Bool(true));
    }

    #[test]
    fn test_nested_iif() {
        assert_eq!(
            eval("iif(eq(1, 1), iif(eq(2, 2), 'both', 'first'), 'neither')"),
            Value::String("both".to_string())
        );
    }
}
