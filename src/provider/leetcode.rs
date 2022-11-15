#![allow(dead_code)]
use crate::provider::Provider;
use crate::io;
use serde::{Serialize, Deserialize};
use serde_json::{Map, Value};
use anyhow::Result;
use std::fmt::Debug;
use std::time::Duration;
use reqwest::{Client, header::{HeaderMap, HeaderValue, self}};
use async_trait::async_trait;
use colored::Colorize;
use tokio::time::interval;
use thiserror::Error;

const LEETCODE_GRAPHQL: &'static str = "https://leetcode.com/graphql";

trait JudgeResult {
    fn get_state(&self) -> &str ;
}

#[derive(Error, Debug)]
enum LeetCodeError {
    #[error("problem does not exist")]
    ProblemNotFound
}

pub struct LeetCode {
    client: Client,
    csrftoken: String,
    leetcode_session: String
}

impl LeetCode {
    pub fn new(csrftoken: String, leetcode_session: String) -> Self {
        Self {
            client: Client::new(),
            csrftoken,
            leetcode_session
        }
    }

    pub fn pretty_test_result(result: TestResult) {
        let runtime = result.status_runtime.unwrap_or_default();
        let memory = result.status_memory.unwrap_or_default();
        let total_correct = result.total_correct.map(|v| v.to_string()).unwrap_or("N/A".to_string());
        let total_testcases = result.total_testcases.map(|v| v.to_string()).unwrap_or("N/A".to_string());
        match (result.status_msg.as_deref(), result.correct_answer) {
            (Some("Accepted"), Some(true)) => {
                println!("{} {}/{}", "✔ Accepted".green(), total_correct.green(), total_testcases.green());
                println!("{} {}, {} {}", "Runtime: ".green(), runtime, "Memory: ".green(), memory);
            },
            (Some("Accepted"), Some(false)) => {
                println!("{} {}/{}", "✘ WA".red(), total_correct.red(), total_testcases.red());
            },
            (Some(msg), None) => {
                println!("{} {}", "✘ ".red(), msg.red());
                if let Some(compile_msg) = result.compile_error {
                    println!("{}", compile_msg.red());
                }
            },
            (_, _) => unreachable!()
        }
    }

    pub fn pretty_submit_result(result: SubmitResult) {
        let runtime = result.status_runtime.unwrap_or_default();
        let memory = result.status_memory.unwrap_or_default();
        let runtime_percent = result.runtime_percentile.unwrap_or_default();
        let memory_percent = result.memory_percentile.unwrap_or_default();
        let total_correct = result.total_correct.map(|v| v.to_string()).unwrap_or("N/A".to_string());
        let total_testcases = result.total_testcases.map(|v| v.to_string()).unwrap_or("N/A".to_string());
        match (result.status_msg.as_deref(), result.run_success) {
            (Some("Accepted"), Some(true)) => {
                println!("{} {}/{}", "✔ Accepted".green(), total_correct.green(), total_testcases.green());
                println!("{} {}/{}, {} {}/{}", "Runtime: ".green(), runtime, runtime_percent, "Memory: ".green(), memory, memory_percent);
            },
            (Some("Accepted"), Some(false)) => {
                println!("{} {}/{}", "✘ WA".red(), total_correct.red(), total_testcases.red());
            },
            (Some(msg), None) => {
                println!("{} {}", "✘ ".red(), msg.red());
                if let Some(compile_msg) = result.compile_error {
                    println!("{}", compile_msg.red());
                }
            },
            (_, _) => unreachable!()
        }
    }

    async fn get_result<T: JudgeResult>(&self, id: &str) -> Result<T>
        where for <'de> T: Deserialize<'de>
    {
        let mut ticker = interval(Duration::from_millis(250));
        let result_url = format!("https://leetcode.com/submissions/detail/{}/check/", id);
        loop {
            ticker.tick().await;
            let result = self.client.get(&result_url)
               .headers(self.headers())
               .send().await?
               .json::<T>().await?;
            if result.get_state() == "SUCCESS" {
                break Ok(result)
            }
        }
    }

    async fn post<REQ: Serialize, RESP>(&self, url: &str, body: &REQ) -> Result<RESP>
        where RESP: for<'de> Deserialize<'de>
    {
        Ok(self.client.post(url)
            .headers(self.headers())
            .json(&body)
            .send().await?
            .json::<RESP>().await?)
    }

    fn headers(&self) -> HeaderMap {
        let mut cookie = std::collections::HashMap::new();
        cookie.insert("csrftoken", self.csrftoken.to_owned());
        cookie.insert("LEETCODE_SESSION", self.leetcode_session.to_owned());
        let cookie = cookie.iter().map(|(k, v)| {
            format!("{}={}", k, v).replace(";", "%3B")
        }).collect::<Vec<_>>()
        .join(";");
            
        let mut headers = HeaderMap::new();
        headers.insert(header::REFERER, HeaderValue::from_static("https://leetcode.com/"));
        headers.insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        headers.insert("x-csrftoken", HeaderValue::from_str(&self.csrftoken).unwrap());
        headers
    }
}

#[derive(Serialize, Debug)]
struct TestRequest {
    data_input: String,
    judge_type: String,
    lang: String,
    question_id: String,
    typed_code: String
}

impl TestRequest {
    fn new(p: &ProblemDetial, lang: &str, typed_code: &str) -> (String, Self) {
        (format!("https://leetcode.com/problems/{}/interpret_solution/", p.title_slug),
            Self {
                data_input: p.example_testcases.to_owned(),
                judge_type: "large".to_string(),
                lang: lang.to_owned(),
                typed_code: typed_code.to_owned(),
                question_id: p.question_id.to_owned()
            })
    }
}

#[derive(Deserialize, Debug)]
struct TestResponse {
    interpret_id: String,
    test_case: String
}

#[derive(Deserialize, Debug)]
pub struct TestResult {
    state: String,
    status_msg: Option<String>,
    status_runtime: Option<String>,
    status_memory: Option<String>,
    correct_answer: Option<bool>,
    compile_error: Option<String>,
    total_correct: Option<u32>,
    total_testcases: Option<u32>
}

impl JudgeResult for TestResult {
    fn get_state(&self) -> &str {
        &self.state
    }
}

#[derive(Serialize)]
struct SubmitRequest {
    lang: String,
    question_id: String,
    typed_code: String
}

impl SubmitRequest {
    fn new(p: &ProblemDetial, lang: &str, typed_code: &str) -> (String, Self) {
        let url = format!("https://leetcode.com/problems/{}/submit/", p.title_slug);
        (url, Self {
            lang: lang.to_owned(),
            question_id: p.question_frontend_id.to_owned(),
            typed_code: typed_code.to_owned()
        })
    }
}
#[derive(Deserialize, Debug)]
struct SubmitResponse {
    submission_id: u32
}

#[derive(Deserialize, Debug)]
pub struct SubmitResult {
    state: String,
    status_msg: Option<String>,
    status_runtime: Option<String>,
    status_memory: Option<String>,
    compile_error: Option<String>,
    total_correct: Option<u32>,
    total_testcases: Option<u32>,
    runtime_percentile: Option<f32>,
    memory_percentile: Option<f32>,
    run_success: Option<bool>
}

impl JudgeResult for SubmitResult {
    fn get_state(&self) -> &str {
        &self.state
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    title: String,
    title_slug: String,
    question_id: String,
    question_frontend_id: String,
    difficulty: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProblemDetial {
    question_id: String,
    question_frontend_id: String,
    title: String,
    title_slug: String,
    code_snippets: Vec<CodeSnippet>,
    example_testcases: String
}

impl ProblemDetial {
    pub fn generate_sinppet(&self, lang: &str) -> Result<()> {
        if let Some(code) = self.get_template(lang) {
            let file_name = format!("{}.{}.{}", self.question_frontend_id, self.title_slug, lang);
            Ok(io::write_to_file(&file_name, &code)?)
        } else {
            Err(anyhow::anyhow!("language do not support"))
        }
    }

    fn get_template(&self, lang: &str) -> Option<String> {
        self.code_snippets.iter().filter(|&p| {
            p.lang_slug == lang
        }).map(|p| {
            p.code.clone()
        }).next()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CodeSnippet {
    lang: String,
    lang_slug: String,
    code: String
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    operation_name: &'static str,
    query: &'static str,
    variables: Map<String, Value>
}

impl Request {
    fn new(operation_name: &'static str, query: &'static str) -> Self {
        Self {
            operation_name,
            query,
            variables: Map::new()
        }
    }

    fn set_variable(mut self, key: &str, value: &str) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }
}

#[derive(Deserialize, Debug)]
struct Response {
    data: Value
}

#[async_trait]
impl Provider for LeetCode {
    type Problem = Problem;
    type ProblemDetial = ProblemDetial;
    type TestResult = TestResult;
    type SubmitResult = SubmitResult;
    fn name(&self) -> &'static str {
        "leetcode"
    }

    async fn get_problem(&self, id: &str) -> Result<ProblemDetial> {
        let title_slug = self.get_problems().await?
            .iter().filter(|p| {
                p.question_frontend_id == id
            })
            .map(|p| p.title_slug.clone())
            .next().ok_or(LeetCodeError::ProblemNotFound)?;
        let query = include_str!("../payload/leetcode/get_problem.txt");
        let req = Request::new("questionData", query)
            .set_variable("titleSlug", &title_slug);
        let resp = self.post::<_, Response>(LEETCODE_GRAPHQL, &req).await?;
        let problem: ProblemDetial = serde_json::value::from_value(resp.data.get("question").unwrap().clone())?;
        Ok(problem)
    }

    async fn get_problems(&self) -> Result<Vec<Problem>> {
        if let Some(problems) = self.load_from_cache() {
            return Ok(problems);
        }
        let query = include_str!("../payload/leetcode/get_problems.txt");
        let body = Request::new("allQuestionsRaw", query);
        let resp = self.post::<_, Response>(LEETCODE_GRAPHQL, &body).await?;
        let problems = resp.data.get("allQuestions").and_then(|problems| {
            Some(problems.as_array().unwrap().into_iter().map(|p| {
                serde_json::value::from_value(p.clone()).unwrap()
            }).collect::<Vec<Problem>>())
        }).unwrap_or(vec![]);
        self.write_to_cache(&problems)?;
        Ok(problems)
    }

    async fn test_code(&self, q: &str, lang: &str, typed_code: &str) -> Result<TestResult> {
        let problem = self.get_problem(q).await?;
        let (url, req) = TestRequest::new(&problem, lang, typed_code);
        let resp = self.post::<_, TestResponse>(&url, &req).await?;
        self.get_result(&resp.interpret_id).await
    }

    async fn submit_code(&self, q: &str, lang: &str, typed_code: &str) -> Result<SubmitResult> {
        let problem = self.get_problem(q).await?;
        let (url, req) = SubmitRequest::new(&problem, lang, typed_code);
        let resp = self.post::<_, SubmitResponse>(&url, &req).await?;
        self.get_result(&resp.submission_id.to_string()).await
    }
}