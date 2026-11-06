mod gui;
mod history;
mod storage;
mod wayland;

use history::History;
use std::env;
use std::path::Path;
use std::io::{self, BufRead, BufReader, Read};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

fn main() -> std::process::ExitCode {
    let mut args = env::args_os().skip(1);
    let result = match (args.next(), args.next(), args.next(), args.next()) {
        (None, None, None, None) => run(History::default(), false),
        (Some(mode), None, None, None) if mode == "--gui" => run(History::default(), true),
        (Some(mode), None, None, None) if mode == "--mostrar" => {
            let activation = std::process::Command::new("gdbus")
                .args([
                    "call", "--session", "--dest", "io.github.altairdeb.Clipper",
                    "--object-path", "/io/github/altairdeb/Clipper",
                    "--method", "org.gtk.Application.Activate", "{}",
                ])
                .output();
            if matches!(activation, Ok(output) if output.status.success()) {
                Ok(())
            } else {
                run(History::default(), true)
            }
        }
        (Some(gui), Some(mode), Some(path), None) if gui == "--gui" && mode == "--persistir" => {
            open_history(Path::new(&path)).and_then(|history| run(history, true))
        }
        (Some(mode), None, None, None) if mode == "--receive" => wayland::receive_selection(),
        (Some(mode), Some(path), None, None) if mode == "--persistir" => {
            open_history(Path::new(&path)).and_then(|history| run(history, false))
        }
        _ => Err(io::Error::other("uso: clipper [--gui] [--persistir ARCHIVO] | --mostrar")),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("clipper: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn open_history(path: &Path) -> io::Result<History> {
    let is_new = !path.try_exists()?;
    let password = rpassword::prompt_password("Frase de paso del historial: ")?;
    if is_new {
        let confirmation = rpassword::prompt_password("Repite la frase de paso: ")?;
        if password != confirmation {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "las frases no coinciden"));
        }
    }
    storage::Storage::open(path, &password).map(History::persistent)
}

fn run(history: History, graphical: bool) -> io::Result<()> {
    let (watcher, output) = wayland::watch()?;
    let history = Arc::new(Mutex::new(history));
    let captures = Arc::clone(&history);
    let capture_running = Arc::new(AtomicBool::new(true));
    let reader_running = Arc::clone(&capture_running);
    let shutting_down = Arc::new(AtomicBool::new(false));
    let reader_shutdown = Arc::clone(&shutting_down);
    let reader = thread::spawn(move || {
        let result = capture_loop(output, &captures);
        reader_running.store(false, Ordering::Relaxed);
        if reader_shutdown.load(Ordering::Relaxed) {
            return;
        }
        if let Err(error) = result {
            eprintln!("clipper: captura detenida ({error}). Revisa la salida de wl-paste.");
        }
    });

    let result = if graphical {
        gui::run(&history)
    } else {
        print_help();
        commands(&history, &capture_running)
    };
    shutting_down.store(true, Ordering::Relaxed);
    drop(watcher);
    drop(reader);
    result
}

fn capture_loop(input: impl Read, history: &Mutex<History>) -> io::Result<()> {
    let mut input = BufReader::new(input);
    loop {
        let text = wayland::read_selection(&mut input)?;
        if let Err(error) = history.lock().unwrap().capture(text) {
            eprintln!("clipper: no se pudo guardar la entrada ({error})");
        }
    }
}

fn print_help() {
    println!("Comandos: lista [busqueda], copia ID, elimina ID, fija ID, desfija ID, pausa, reanuda, vacia, estado, ayuda, salir");
}

fn commands(history: &Mutex<History>, capture_running: &AtomicBool) -> io::Result<()> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        let (command, argument) = line.trim().split_once(' ').unwrap_or((line.trim(), ""));
        match command {
            "lista" => {
                let query = argument.trim().to_lowercase();
                let entries = history.lock().unwrap().entries()?;
                for (id, text, pinned) in entries {
                    if text.to_lowercase().contains(&query) {
                        let preview: String = text.chars().take(100).flat_map(char::escape_debug).collect();
                        let label = if pinned { "[fijada] " } else { "" };
                        println!("{id}: {label}{preview}");
                    }
                }
            }
            "copia" => {
                let text = match parse_id(argument) {
                    Ok(id) => history.lock().unwrap().get(id)?,
                    Err(_) => None,
                };

                match text {
                    Some(text) => match wayland::restore(&text) {
                        Ok(()) => println!("Texto restaurado."),
                        Err(error) => eprintln!("clipper: {error}"),
                    },
                    None => eprintln!("Entrada desconocida. Usa lista para ver los identificadores."),
                }
            }
            "elimina" | "fija" | "desfija" => {
                let result = parse_id(argument).and_then(|id| {
                    let mut history = history.lock().unwrap();
                    if command == "elimina" {
                        return history.remove(id);
                    }
                    if history.pin(id, command == "fija")? {
                        Ok(())
                    } else {
                        Err(io::Error::other("entrada desconocida"))
                    }
                });
                if let Err(error) = result {
                    eprintln!("clipper: {error}");
                }
            }
            "ayuda" => print_help(),
            "estado" => {
                let history = history.lock().unwrap();
                let status = if !capture_running.load(Ordering::Relaxed) {
                    "detenida"
                } else if history.paused {
                    "en pausa"
                } else {
                    "activa"
                };
                println!("Captura {status}. Entradas: {}.", history.len()?);
            }
            "pausa" => history.lock().unwrap().paused = true,
            "reanuda" => history.lock().unwrap().paused = false,
            "vacia" => {
                if let Err(error) = history.lock().unwrap().clear() {
                    eprintln!("clipper: {error}");
                }
            }
            "salir" => break,
            "" => {}
            _ => eprintln!("Comando desconocido."),
        }
    }
    Ok(())
}

fn parse_id(argument: &str) -> io::Result<u64> {
    argument.trim().parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .map(|id| id as u64)
        .ok_or_else(|| io::Error::other("indica un identificador valido"))
}
