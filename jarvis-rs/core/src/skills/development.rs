//! Skill development service for autonomous skill generation.

use crate::intent::types::IntentParameters;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a skill that can be developed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Name of the skill
    pub name: String,
    /// Description of what the skill does
    pub description: String,
    /// Programming language (rust, python, javascript, etc.)
    pub language: String,
    /// Type of skill (api, library, component, script, console)
    pub skill_type: String,
    /// Source code of the skill
    pub code: String,
    /// Test code for the skill
    pub test_code: Option<String>,
    /// Dependencies required by the skill
    pub dependencies: Vec<String>,
    /// Parameters/configuration for the skill
    pub parameters: HashMap<String, String>,
    /// Version of the skill
    pub version: String,
}

/// Result of skill development.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDevelopmentResult {
    /// The developed skill definition
    pub skill: SkillDefinition,
    /// Whether development was successful
    pub success: bool,
    /// Any errors encountered during development
    pub errors: Vec<String>,
    /// Warnings generated during development
    pub warnings: Vec<String>,
}

/// Trait for skill development service.
#[async_trait::async_trait]
pub trait SkillDevelopmentService: Send + Sync {
    /// Generates a skill based on requirements.
    async fn generate_skill(
        &self,
        requirements: &str,
        parameters: &IntentParameters,
    ) -> Result<SkillDevelopmentResult>;
}

/// LLM-based skill development service.
///
/// This service uses an LLM to generate skill code based on requirements.
/// In a real implementation, this would integrate with the ModelClient.
pub struct LLMSkillDevelopmentService {
    /// Default language for skill generation
    default_language: String,
    /// Default skill type
    default_skill_type: String,
}

impl LLMSkillDevelopmentService {
    /// Creates a new LLM-based skill development service.
    pub fn new(default_language: String, default_skill_type: String) -> Self {
        Self {
            default_language,
            default_skill_type,
        }
    }

    /// Generates a skill name from requirements.
    fn generate_skill_name(&self, requirements: &str) -> String {
        // Simple name generation - in production, use LLM
        let name = requirements
            .to_lowercase()
            .chars()
            .take(30)
            .collect::<String>()
            .replace(' ', "_")
            .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
        
        if name.is_empty() {
            "generated_skill".to_string()
        } else {
            format!("{}_skill", name)
        }
    }

    /// Generates basic code template based on language and type.
    fn generate_code_template(
        &self,
        language: &str,
        skill_type: &str,
        name: &str,
    ) -> String {
        match (language, skill_type) {
            ("rust", "api") => format!(
                r#"// {} API Skill
use std::collections::HashMap;
use serde::{{Deserialize, Serialize}};

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {{
    pub message: String,
    pub status: String,
}}

pub fn execute(params: HashMap<String, String>) -> Result<Response, String> {{
    Ok(Response {{
        message: "Skill {} executed successfully".to_string(),
        status: "success".to_string(),
    }})
}}
"#,
                name, name
            ),
            ("rust", "library") => format!(
                r#"// {} Library Skill
pub struct {} {{
    // Add fields as needed
}}

impl {} {{
    pub fn new() -> Self {{
        Self {{
            // Initialize fields
        }}
    }}
    
    pub fn execute(&self) -> Result<String, String> {{
        Ok("Skill {} executed successfully".to_string())
    }}
}}
"#,
                name, name, name, name
            ),
            ("python", "api") => format!(
                r#"# {} API Skill
from typing import Dict, Any

def execute(params: Dict[str, Any]) -> Dict[str, Any]:
    return {{
        "message": "Skill {} executed successfully",
        "status": "success"
    }}
"#,
                name, name
            ),
            ("javascript", "api") => format!(
                r#"// {} API Skill
function execute(params) {{
    return {{
        message: "Skill {} executed successfully",
        status: "success"
    }};
}}

module.exports = {{ execute }};
"#,
                name, name
            ),
            _ => format!(
                r#"// {} Skill
// Generated skill code for {} language and {} type
// TODO: Implement skill logic
"#,
                name, language, skill_type
            ),
        }
    }

    /// Generates test code template.
    fn generate_test_template(&self, language: &str, name: &str) -> String {
        match language {
            "rust" => format!(
                r#"#[cfg(test)]
mod tests {{
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_{}_execution() {{
        let params = HashMap::new();
        let result = execute(params);
        assert!(result.is_ok());
    }}
}}
"#,
                name.replace('-', "_")
            ),
            "python" => format!(
                r#"import unittest
from {} import execute

class Test{}(unittest.TestCase):
    def test_execution(self):
        params = {{}}
        result = execute(params)
        self.assertIn("status", result)
        self.assertEqual(result["status"], "success")

if __name__ == "__main__":
    unittest.main()
"#,
                name, name
            ),
            "javascript" => format!(
                r#"const {{ execute }} = require('./{}');

describe('{}', () => {{
    test('should execute successfully', () => {{
        const params = {{}};
        const result = execute(params);
        expect(result.status).toBe('success');
    }});
}});
"#,
                name, name
            ),
            _ => format!(
                r#"// Test code for {} skill
// TODO: Implement tests
"#,
                name
            ),
        }
    }
}

#[async_trait::async_trait]
impl SkillDevelopmentService for LLMSkillDevelopmentService {
    async fn generate_skill(
        &self,
        requirements: &str,
        parameters: &IntentParameters,
    ) -> Result<SkillDevelopmentResult> {
        let language = parameters
            .language
            .as_ref()
            .unwrap_or(&self.default_language)
            .clone();
        let skill_type = parameters
            .skill_type
            .as_ref()
            .unwrap_or(&self.default_skill_type)
            .clone();
        let description = parameters
            .description
            .as_ref()
            .unwrap_or(&requirements.to_string())
            .clone();

        let name = self.generate_skill_name(requirements);
        let code = self.generate_code_template(&language, &skill_type, &name);
        let test_code = Some(self.generate_test_template(&language, &name));

        let skill = SkillDefinition {
            name: name.clone(),
            description,
            language,
            skill_type,
            code,
            test_code,
            dependencies: vec![],
            parameters: HashMap::new(),
            version: "1.0.0".to_string(),
        };

        Ok(SkillDevelopmentResult {
            skill,
            success: true,
            errors: vec![],
            warnings: vec!["Generated code is a template and may need customization".to_string()],
        })
    }
}

impl Default for LLMSkillDevelopmentService {
    fn default() -> Self {
        Self::new("rust".to_string(), "library".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::types::IntentParameters;

    #[tokio::test]
    async fn test_generate_rust_api_skill() {
        let service = LLMSkillDevelopmentService::new("rust".to_string(), "api".to_string());
        let mut params = IntentParameters::default();
        params.language = Some("rust".to_string());
        params.skill_type = Some("api".to_string());
        params.description = Some("A REST API for managing products".to_string());

        let result = service
            .generate_skill("Create a REST API", &params)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.skill.language, "rust");
        assert_eq!(result.skill.skill_type, "api");
        assert!(result.skill.code.contains("pub fn execute"));
        assert!(result.skill.test_code.is_some());
    }

    #[tokio::test]
    async fn test_generate_python_skill() {
        let service = LLMSkillDevelopmentService::new("python".to_string(), "api".to_string());
        let mut params = IntentParameters::default();
        params.language = Some("python".to_string());

        let result = service
            .generate_skill("Create a Python API", &params)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.skill.language, "python");
        assert!(result.skill.code.contains("def execute"));
    }
}
