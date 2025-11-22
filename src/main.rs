use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, stdout};

#[derive(Parser)]
#[command(name = "ruc_auth")]
#[command(about = "一个支持命令行参数和TUI的终端应用", long_about = None)]
struct Cli {
    /// 启用TUI模式
    #[arg(short, long)]
    tui: bool,

    /// 详细输出
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 执行某个操作
    Run {
        /// 操作名称
        #[arg(short, long)]
        name: Option<String>,
    },
    /// 显示配置信息
    Config {
        /// 配置文件路径
        #[arg(short, long)]
        file: Option<String>,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // 如果指定了 --tui 参数，进入TUI模式
    if cli.tui {
        run_tui()?;
    } else {
        // 否则执行命令行模式
        run_cli(&cli)?;
    }

    Ok(())
}

fn run_cli(cli: &Cli) -> io::Result<()> {
    if cli.verbose {
        println!("详细模式已启用");
    }

    match &cli.command {
        Some(Commands::Run { name }) => {
            if let Some(n) = name {
                println!("执行操作: {}", n);
            } else {
                println!("执行默认操作");
            }
        }
        Some(Commands::Config { file }) => {
            if let Some(f) = file {
                println!("配置文件路径: {}", f);
            } else {
                println!("使用默认配置");
            }
        }
        None => {
            println!("使用 --help 查看帮助信息");
            println!("使用 --tui 进入TUI模式");
        }
    }

    Ok(())
}

fn run_tui() -> io::Result<()> {
    // 设置终端为原始模式
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| ui(f))?;
        should_quit = handle_events()?;
    }

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn handle_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
                    KeyCode::Char('h') => {
                        // 可以在这里添加帮助信息显示逻辑
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(false)
}

fn ui(f: &mut Frame) {
    let size = f.size();

    // 创建布局
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // 标题区域
            Constraint::Min(0),    // 主内容区域
            Constraint::Length(3), // 底部帮助区域
        ])
        .split(size);

    // 标题
    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "RUC Auth",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::raw("终端应用示例")]),
    ])
    .block(Block::default().borders(Borders::ALL).title("应用标题"))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    f.render_widget(title, chunks[0]);

    // 主内容区域
    let content = Paragraph::new(vec![
        Line::from("欢迎使用 TUI 模式！"),
        Line::from(""),
        Line::from(vec![
            Span::styled("快捷键:", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  • 按 'q' 或 'Esc' 退出"),
        Line::from("  • 按 'h' 显示帮助"),
    ])
    .block(Block::default().borders(Borders::ALL).title("内容"))
    .wrap(Wrap { trim: true });
    f.render_widget(content, chunks[1]);

    // 底部帮助信息
    let help = Paragraph::new("按 'q' 退出 | 按 'h' 帮助")
        .block(Block::default().borders(Borders::ALL).title("帮助"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);
}
