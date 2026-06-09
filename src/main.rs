use clap::Parser;
use mimalloc::MiMalloc;

use dc::{load_configuration, print_report, run_analysis};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug)]
#[command(name = "dc", about = "Поиск мертвого кода в Django проектах")]
struct CommandLineArguments {
    #[arg(short, long, default_value = ".")]
    target_path: String,

    #[arg(short, long, default_value = ".dc.toml")]
    config_path: String,

    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

/// Инициализирует процесс анализа и выводит результаты.
///
/// :return: Статус завершения программы.
fn main() -> std::process::ExitCode {
    let command_line_arguments = CommandLineArguments::parse();
    let analyzer_configuration = load_configuration(&command_line_arguments.config_path);
    let analysis_report = run_analysis(&command_line_arguments.target_path, &analyzer_configuration);
    print_report(&analysis_report, command_line_arguments.verbose);
    std::process::ExitCode::SUCCESS
}
