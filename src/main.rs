mod provider;
mod io;
use clap::Parser;
use dotenv::dotenv;
use provider::{LeetCode, Provider};
use std::env;
#[derive(clap::Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand, Debug)]
enum Action {
    Get(GetArgs),
    Test(SubmitArgs),
    Submit(SubmitArgs),
}

#[derive(clap::Args, Debug)]
struct GetArgs {
    id: String,
    lang: String,
}

#[derive(clap::Args, Debug)]
struct SubmitArgs {
    id: String,
    lang: String,
    file: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let command = Cli::parse();
    let csrftoken = env::var("csrftoken").unwrap_or_default();
    let leetcode_session = env::var("LEETCODE_SESSION").unwrap_or_default();
    let client = LeetCode::new(csrftoken, leetcode_session);
    match command.action {
        Action::Get(args) => {
            let problem = client.get_problem(&args.id).await.unwrap();
            let _ = problem.generate_sinppet(&args.lang);
        },
        Action::Test(args) => {
            let code = io::read_from_file(&args.file)?;
            client.test_code(&args.id, &args.lang, &code).await
                .map(LeetCode::pretty_test_result)?;
        },
        Action::Submit(args) => {
            let code = io::read_from_file(&args.file)?;
            client.submit_code(&args.id, &args.lang, &code).await
                .map(LeetCode::pretty_submit_result)?;
        },
    }

    Ok(())
}
