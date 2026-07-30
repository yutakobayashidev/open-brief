use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use clap::{ArgAction, Args, Parser, Subcommand};
use openbrief_app::{
    AppPaths, AttentionService, CollectorStatus, ContextDetail, ContextService, IngestOutcome,
    RecordingStatus, run_proposal_mcp_server, run_watch,
};
use openbrief_core::{ActivitySlice, ContextBrief, FocusState, ObservationBatch};
use serde::Serialize;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime, Time};

const DATE_FORMAT: &[BorrowedFormatItem<'_>] = format_description!("[year]-[month]-[day]");
const TIME_FORMAT: &[BorrowedFormatItem<'_>] = format_description!("[hour]:[minute]");
const EVENT_TIME_FORMAT: &[BorrowedFormatItem<'_>] =
    format_description!("[hour]:[minute]:[second]");

#[derive(Debug, Parser)]
#[command(name = "openbrief", version, about = "Recall recent work context")]
struct Cli {
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    no_color: bool,
    #[arg(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Enable,
    Disable,
    Status(JsonArgs),
    Recent(RecentArgs),
    Around(AroundArgs),
    Today(TodayArgs),
    Pause(PauseArgs),
    Resume,
    Delete(DeleteArgs),
    Ingest(IngestArgs),
    #[command(hide = true)]
    Mcp(McpArgs),
    #[command(hide = true)]
    Watch,
}

#[derive(Debug, Args)]
struct JsonArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RecentArgs {
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(i64).range(5..=120))]
    minutes: i64,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct AroundArgs {
    time: String,
    #[arg(long)]
    date: Option<String>,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(i64).range(5..=120))]
    minutes: i64,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct TodayArgs {
    #[arg(long)]
    date: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct PauseArgs {
    #[arg(long = "for")]
    duration: Option<String>,
}

#[derive(Debug, Args)]
struct DeleteArgs {
    #[arg(long, required_unless_present = "date", conflicts_with = "date")]
    today: bool,
    #[arg(long, required_unless_present = "today")]
    date: Option<String>,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    no_input: bool,
}

#[derive(Debug, Args)]
struct IngestArgs {
    /// `ObservationBatch` JSON file. Reads stdin when omitted.
    file: Option<std::path::PathBuf>,
}

#[derive(Debug, Args)]
struct McpArgs {
    #[command(subcommand)]
    command: McpCommand,
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    Serve(McpServeArgs),
}

#[derive(Debug, Args)]
struct McpServeArgs {
    #[arg(long)]
    database: Option<std::path::PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match execute(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("openbrief: {error}");
            ExitCode::from(1)
        }
    }
}

fn execute(cli: Cli) -> Result<(), CliError> {
    if matches!(cli.command, Command::Watch) {
        return run_watch().map_err(CliError::Watch);
    }
    if let Command::Mcp(args) = &cli.command {
        return run_mcp(args);
    }
    if let Command::Ingest(args) = &cli.command {
        return run_ingest(args);
    }
    let service = ContextService::discover()?;
    match cli.command {
        Command::Enable => {
            let executable = std::env::current_exe().map_err(CliError::CurrentExecutable)?;
            let config = service.enable(&executable)?;
            println!("Context recall enabled.");
            println!("Mode: metadata only");
            println!("Excluded apps: {}", config.capture.excluded_apps.join(", "));
            println!("Retention: {} days", config.retention_days);
        }
        Command::Disable => {
            service.disable()?;
            println!("Context recall disabled. Saved data was not deleted.");
        }
        Command::Status(args) => {
            let status = service.status()?;
            if args.json {
                print_json(&Envelope::new(status))?;
            } else {
                print_status(&status);
            }
        }
        Command::Recent(args) => {
            let brief = service.recent(now_local(), args.minutes)?;
            print_brief(&brief, args.json)?;
        }
        Command::Around(args) => {
            let now = now_local();
            let date = parse_date_or(args.date.as_deref(), now.date())?;
            let time = Time::parse(&args.time, TIME_FORMAT).map_err(CliError::Time)?;
            let anchor = date.with_time(time).assume_offset(now.offset());
            let detail = service.around(anchor, args.minutes)?;
            print_detail(&detail, args.json)?;
        }
        Command::Today(args) => {
            let now = now_local();
            let date = parse_date_or(args.date.as_deref(), now.date())?;
            let brief = service.today(date, now.offset())?;
            print_brief(&brief, args.json)?;
        }
        Command::Pause(args) => {
            let until = args
                .duration
                .as_deref()
                .map(parse_duration)
                .transpose()?
                .map(|duration| now_local() + duration);
            service.pause(until)?;
            match until {
                Some(until) => println!("Context recall paused until {until}."),
                None => println!("Context recall paused."),
            }
        }
        Command::Resume => {
            service.resume()?;
            println!("Context recall resumed.");
        }
        Command::Delete(args) => {
            confirm_delete(&args)?;
            let now = now_local();
            let date = if args.today {
                now.date()
            } else {
                parse_date_or(args.date.as_deref(), now.date())?
            };
            let deleted = service.delete_date(date, now.offset())?;
            println!("Deleted {deleted} context segments for {date}.");
        }
        Command::Ingest(_) => unreachable!("ingest handled before context service discovery"),
        Command::Mcp(_) => unreachable!("MCP handled before path discovery"),
        Command::Watch => unreachable!("watch handled before path discovery"),
    }
    Ok(())
}

fn run_mcp(args: &McpArgs) -> Result<(), CliError> {
    match &args.command {
        McpCommand::Serve(args) => {
            let database = args
                .database
                .clone()
                .map_or_else(|| AppPaths::discover().map(|paths| paths.database_file), Ok)?;
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(CliError::Runtime)?
                .block_on(run_proposal_mcp_server(database))
                .map_err(CliError::Mcp)
        }
    }
}

fn run_ingest(args: &IngestArgs) -> Result<(), CliError> {
    let mut raw = String::new();
    if let Some(file) = &args.file {
        raw = std::fs::read_to_string(file).map_err(CliError::Input)?;
    } else {
        io::stdin()
            .read_to_string(&mut raw)
            .map_err(CliError::Input)?;
    }
    let batch: ObservationBatch = serde_json::from_str(&raw).map_err(CliError::Json)?;
    let paths = AppPaths::discover()?;
    let mut service = AttentionService::open(paths.database_file).map_err(CliError::Attention)?;
    let outcome = service.ingest(&batch).map_err(CliError::Attention)?;
    match outcome {
        IngestOutcome::Inserted => println!("Ingested ObservationBatch {}.", batch.id),
        IngestOutcome::AlreadyPresent => {
            println!("ObservationBatch {} was already present.", batch.id);
        }
    }
    Ok(())
}

fn print_status(status: &CollectorStatus) {
    let recording = match status.recording {
        RecordingStatus::Active => "active",
        RecordingStatus::Paused => "paused",
        RecordingStatus::Disabled => "disabled",
    };
    println!("Recording: {recording}");
    println!(
        "Source: {}",
        if status.source_available {
            "available"
        } else {
            "unavailable"
        }
    );
    if let Some(last) = status.last_window_event_at {
        println!("Last window event: {last}");
    }
    if let Some(until) = status.paused_until {
        println!("Paused until: {until}");
    }
}

fn print_brief(brief: &ContextBrief, json: bool) -> Result<(), CliError> {
    if json {
        return print_json(&Envelope::new(brief));
    }
    println!("Context {} – {}", brief.start, brief.end);
    if brief.slices.is_empty() {
        println!("  No context was recorded in this range.");
        return Ok(());
    }
    for slice in &brief.slices {
        print_slice(slice)?;
    }
    Ok(())
}

fn print_detail(detail: &ContextDetail, json: bool) -> Result<(), CliError> {
    if json {
        return print_json(&Envelope::new(detail));
    }
    println!("Context {} – {}", detail.start, detail.end);
    if detail.segments.is_empty() {
        println!("  No context was recorded in this range.");
        return Ok(());
    }
    for segment in &detail.segments {
        let at = segment
            .started_at
            .format(EVENT_TIME_FORMAT)
            .map_err(CliError::Format)?;
        match segment.state {
            FocusState::Observed => {
                println!("{at}  {}", segment.app_id().unwrap_or("unknown"));
            }
            _ => println!("{at}  {}", state_label(&segment.state)),
        }
    }
    Ok(())
}

fn print_slice(slice: &ActivitySlice) -> Result<(), CliError> {
    let start = slice.start.format(TIME_FORMAT).map_err(CliError::Format)?;
    let end = slice.end.format(TIME_FORMAT).map_err(CliError::Format)?;
    println!("{start}–{end}");
    for duration in &slice.durations {
        let minutes = duration.seconds / 60;
        let seconds = duration.seconds % 60;
        match duration.state {
            FocusState::Observed => println!(
                "  {} {minutes}m {seconds:02}s",
                duration.app_id().unwrap_or("unknown")
            ),
            _ => println!(
                "  {} {minutes}m {seconds:02}s",
                state_label(&duration.state)
            ),
        }
    }
    Ok(())
}

fn state_label(state: &FocusState) -> &'static str {
    match state {
        FocusState::Observed => "observed",
        FocusState::Excluded => "private / excluded",
        FocusState::Idle => "idle",
        FocusState::Locked => "locked",
        FocusState::Paused => "paused",
        FocusState::Disabled => "disabled",
        FocusState::SourceUnavailable => "source unavailable",
    }
}

fn parse_date_or(value: Option<&str>, default: Date) -> Result<Date, CliError> {
    value.map_or(Ok(default), |value| {
        Date::parse(value, DATE_FORMAT).map_err(CliError::Date)
    })
}

fn parse_duration(value: &str) -> Result<Duration, CliError> {
    let (number, unit) = value.split_at(
        value
            .find(|character: char| !character.is_ascii_digit())
            .ok_or_else(|| CliError::InvalidDuration(value.to_owned()))?,
    );
    let amount: i64 = number
        .parse()
        .map_err(|_| CliError::InvalidDuration(value.to_owned()))?;
    if amount <= 0 {
        return Err(CliError::InvalidDuration(value.to_owned()));
    }
    match unit {
        "m" => Ok(Duration::minutes(amount)),
        "h" => Ok(Duration::hours(amount)),
        _ => Err(CliError::InvalidDuration(value.to_owned())),
    }
}

fn confirm_delete(args: &DeleteArgs) -> Result<(), CliError> {
    if args.no_input {
        return if args.force {
            Ok(())
        } else {
            Err(CliError::NonInteractiveDeleteRequiresForce)
        };
    }
    if !io::stdin().is_terminal() {
        return Err(CliError::NonInteractiveDeleteRequiresFlags);
    }
    print!("Delete the selected day of OpenBrief context? [y/N] ");
    io::stdout().flush().map_err(CliError::Output)?;
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(CliError::Output)?;
    if matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        Ok(())
    } else {
        Err(CliError::DeleteCancelled)
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    serde_json::to_writer_pretty(io::stdout().lock(), value).map_err(CliError::Json)?;
    println!();
    Ok(())
}

fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

#[derive(Debug, Serialize)]
struct Envelope<T> {
    schema_version: u32,
    data: T,
}

impl<T> Envelope<T> {
    fn new(data: T) -> Self {
        Self {
            schema_version: 1,
            data,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error(transparent)]
    Query(#[from] openbrief_app::QueryError),
    #[error("collector failed: {0}")]
    Watch(openbrief_app::WatchError),
    #[error("MCP server failed: {0}")]
    Mcp(openbrief_app::McpServerError),
    #[error(transparent)]
    Paths(#[from] openbrief_app::PathsError),
    #[error("could not create async runtime: {0}")]
    Runtime(std::io::Error),
    #[error("could not read input: {0}")]
    Input(std::io::Error),
    #[error("attention store failed: {0}")]
    Attention(openbrief_app::AttentionError),
    #[error("could not determine current executable: {0}")]
    CurrentExecutable(std::io::Error),
    #[error("invalid date: {0}")]
    Date(time::error::Parse),
    #[error("invalid time: {0}")]
    Time(time::error::Parse),
    #[error("could not format time: {0}")]
    Format(time::error::Format),
    #[error("invalid duration {0}; use values such as 30m or 2h")]
    InvalidDuration(String),
    #[error("non-interactive delete requires both --force and --no-input")]
    NonInteractiveDeleteRequiresFlags,
    #[error("--no-input also requires --force")]
    NonInteractiveDeleteRequiresForce,
    #[error("delete cancelled")]
    DeleteCancelled,
    #[error("output failed: {0}")]
    Output(std::io::Error),
    #[error("could not encode JSON output: {0}")]
    Json(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn duration_parser_accepts_minutes_and_hours() {
        assert_eq!(parse_duration("30m").unwrap(), Duration::minutes(30));
        assert_eq!(parse_duration("2h").unwrap(), Duration::hours(2));
        assert!(parse_duration("30").is_err());
        assert!(parse_duration("0m").is_err());
    }

    #[test]
    fn command_tree_accepts_context_queries() {
        Cli::try_parse_from(["openbrief", "recent", "--minutes", "20"]).unwrap();
        Cli::try_parse_from(["openbrief", "around", "14:00", "--json"]).unwrap();
        Cli::try_parse_from(["openbrief", "today", "--date", "2026-07-29"]).unwrap();
        Cli::try_parse_from([
            "openbrief",
            "mcp",
            "serve",
            "--database",
            "/tmp/openbrief.sqlite3",
        ])
        .unwrap();
        Cli::try_parse_from(["openbrief", "ingest", "observations.json"]).unwrap();
    }

    #[test]
    fn delete_requires_exactly_one_date_target() {
        assert!(Cli::try_parse_from(["openbrief", "delete"]).is_err());
        assert!(Cli::try_parse_from(["openbrief", "delete", "--force"]).is_err());
        assert!(
            Cli::try_parse_from(["openbrief", "delete", "--today", "--date", "2026-07-29"])
                .is_err()
        );
        Cli::try_parse_from(["openbrief", "delete", "--today", "--force", "--no-input"]).unwrap();
    }

    #[test]
    fn json_timestamps_are_rfc3339_strings() {
        let status = CollectorStatus {
            schema_version: 1,
            recording: RecordingStatus::Active,
            last_window_event_at: Some(datetime!(2026-07-30 02:55:42 +09:00)),
            paused_until: None,
            source_available: true,
        };

        let json = serde_json::to_string(&Envelope::new(status)).unwrap();
        assert!(json.contains(r#""2026-07-30T02:55:42+09:00""#));
    }
}
