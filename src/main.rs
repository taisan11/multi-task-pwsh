use std::{io,process::Command};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        std::process::exit(1);
    }
    
    let mut cmd = Command::new("pwsh");
    cmd.args(["-NoProfile", "-c", &args[1..].join(" ")]);
    
    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}