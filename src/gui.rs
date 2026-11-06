use crate::{parse_id, wayland, History};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub fn run(history: &Mutex<History>) -> io::Result<()> {
    let mut window = Command::new("/usr/bin/python3")
        .args(["-c", include_str!("window.py")])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut input = window.stdin.take().unwrap();
    let output = BufReader::new(window.stdout.take().unwrap());
    let result = serve(history, output, &mut input);
    drop(input);
    if result.is_err() {
        let _ = window.kill();
    }
    let status = window.wait()?;
    result?;
    if !status.success() {
        return Err(io::Error::other("no se pudo abrir la interfaz GTK de Clipper"));
    }
    Ok(())
}

fn serve(
    history: &Mutex<History>,
    output: impl BufRead,
    input: &mut impl Write,
) -> io::Result<()> {
    for line in output.lines() {
        let line = line?;
        if line == "lista" {
            match history.lock().unwrap().entries() {
                Ok(entries) => {
                    writeln!(input, "{}", entries.len())?;
                    for (id, text, pinned) in entries {
                        let preview: String = text.chars().take(100).flat_map(char::escape_debug).collect();
                        let label = if pinned { "Fijada" } else { "" };
                        writeln!(input, "{id}\t{label}\t{preview}")?;
                    }
                }
                Err(error) => writeln!(input, "{error}")?,
            }
        } else if line == "pega" {
            let sent = Command::new("ydotool")
                .args(["key", "29:1", "47:1", "47:0", "29:0"])
                .stdout(Stdio::null())
                .status();
            if matches!(sent, Ok(status) if status.success()) {
                writeln!(input, "OK")?;
            } else {
                let message = "Texto copiado. Pegalo con Ctrl+V. El pegado automático requiere ydotoold.";
                eprintln!("Clipper: {message}");
                writeln!(input, "{message}")?;
            }
        } else {
            let (command, argument) = line.split_once(' ').unwrap_or((&line, ""));
            match apply(history, command, argument) {
                Ok(message) => writeln!(input, "{message}")?,
                Err(error) => writeln!(input, "{error}")?,
            }
        }
        input.flush()?;
    }
    Ok(())
}

fn apply(history: &Mutex<History>, command: &str, argument: &str) -> io::Result<&'static str> {
    let id = parse_id(argument)?;
    if command == "copia" {
        let text = history.lock().unwrap().get(id)?
            .ok_or_else(|| io::Error::other("La entrada ya no está en el historial."))?;
        wayland::restore(&text)?;
        return Ok("OK");
    }

    let mut history = history.lock().unwrap();
    match command {
        "elimina" => {
            history.remove(id)?;
            Ok("Entrada eliminada.")
        }
        "fija" | "desfija" if history.pin(id, command == "fija")? => {
            Ok(if command == "fija" { "Entrada fijada." } else { "Entrada desfijada." })
        }
        "fija" | "desfija" => Err(io::Error::other("La entrada ya no está en el historial.")),
        _ => Err(io::Error::other("Acción desconocida.")),
    }
}
