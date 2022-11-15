mod leetcode;
pub use leetcode::{
    LeetCode,
    TestResult,
    SubmitResult
};
use anyhow::Result;
use async_trait::async_trait;
use crate::io;

#[async_trait]
pub trait Provider {
    type Problem: serde::Serialize + for<'de> serde::Deserialize<'de>;
    type ProblemDetial;
    type TestResult;
    type SubmitResult;
    fn name(&self) -> &'static str;
    async fn get_problem(&self, q: &str) -> Result<Self::ProblemDetial>;
    async fn get_problems(&self) -> Result<Vec<Self::Problem>>;
    async fn test_code(&self, q: &str, lang: &str, typed_code: &str) -> Result<Self::TestResult>;
    async fn submit_code(&self, q: &str, lang: &str, typed_code: &str) -> Result<Self::SubmitResult>;
    fn load_from_cache(&self) -> Option<Vec<Self::Problem>> {
        let mut path = io::get_cache_folder(self.name())?;
        path.push("problems.json");
        let json = io::read_from_file(path).ok()?;
        Some(serde_json::from_str::<Vec<Self::Problem>>(&json).ok()?)
    }

    fn write_to_cache(&self, problems: &Vec<Self::Problem>) -> Result<()> {
        let json = serde_json::to_string(problems)?;
        let mut path = io::get_cache_folder(self.name())
            .ok_or(anyhow::anyhow!("cache folder not exist"))?;
        path.push("problems.json");
        Ok(io::write_to_file(path, &json)?)
    }
}