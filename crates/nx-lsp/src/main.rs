use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.as_slice() != ["--stdio"] {
        eprintln!("Usage: nx-lsp --stdio");
        return ExitCode::from(2);
    }

    nx_lsp::run_stdio().await;
    ExitCode::SUCCESS
}
