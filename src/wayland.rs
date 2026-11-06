use crate::history::MAX_BYTES;
use std::env;
use std::io::{self, Read, Write};
use std::process::{Child, ChildStdout, Command, Stdio};

pub fn receive_selection() -> io::Result<()> {
    if !matches!(env::var("CLIPBOARD_STATE").as_deref(), Ok("data")) {
        return Ok(());
    }

    let mut bytes = Vec::new();
    io::stdin().take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > MAX_BYTES || std::str::from_utf8(&bytes).is_err() {
        return Ok(());
    }

    let mut output = io::stdout().lock();
    output.write_all(&(bytes.len() as u32).to_be_bytes())?;
    output.write_all(&bytes)
}

pub struct Watcher(Child);

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn watch() -> io::Result<(Watcher, ChildStdout)> {
    if !cfg!(target_os = "linux")
        || (env::var_os("WAYLAND_DISPLAY").is_none() && env::var_os("WAYLAND_SOCKET").is_none())
    {
        return Err(io::Error::other("se requiere una sesion Linux con Wayland"));
    }

    let mut child = Command::new("wl-paste")
            .args(["--type", "text", "--watch"])
            .arg(env::current_exe()?)
            .arg("--receive")
            .env_remove("CLIPBOARD_STATE")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| io::Error::other(format!("no se pudo iniciar wl-paste: {error}")))?;
    let output = child.stdout.take().unwrap();
    Ok((Watcher(child), output))
}

pub fn read_selection(input: &mut impl Read) -> io::Result<String> {
    let mut size = [0; 4];
    input.read_exact(&mut size)?;
    let size = u32::from_be_bytes(size) as usize;
    if size == 0 || size > MAX_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "tamano invalido"));
    }

    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(io::Error::other)
}

pub fn restore(text: &str) -> io::Result<()> {
    let mut child = Command::new("wl-copy")
        .args(["--type", "text/plain;charset=utf-8"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;

    let written = child.stdin.take().unwrap().write_all(text.as_bytes());
    let status = child.wait()?;
    written?;
    if !status.success() {
        return Err(io::Error::other(format!("wl-copy termino con {status}")));
    }
    Ok(())
}
