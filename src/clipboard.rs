//! A separate owner keeps every clipboard format available after the editor closes.
use anyhow::{Result, anyhow, ensure};
use relm4::gtk::{self, gdk, glib, prelude::*};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

/// Exported files are kept this long so pastes and clipboard managers can still read them.
const EXPORT_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

pub fn numbered_text(comments: &[String]) -> String {
    comments
        .iter()
        .enumerate()
        .map(|(index, text)| format!("{}. {}", index + 1, text.replace('\n', "\n   ")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Markdown companion for the screenshot, so AI chats read comments as text.
pub fn comments_markdown(image_name: &str, comments: &[String]) -> String {
    format!(
        "Comments for the numbered regions in {image_name}:\n\n{}\n",
        numbered_text(comments)
    )
}

/// Builds a CRLF-terminated `text/uri-list`.
fn uri_list(paths: &[PathBuf]) -> Result<String> {
    paths
        .iter()
        .map(|path| Ok(format!("{}\r\n", glib::filename_to_uri(path, None)?)))
        .collect()
}

/// GNOME/GTK file managers' copied-files format for the same URIs.
fn gnome_copied_files(uri_list: &str) -> String {
    format!("copy\n{}", uri_list.trim_end().replace("\r\n", "\n"))
}

/// Writes `rocks-<time>.png` and `.md` so chats can attach the pair as files.
fn export_files(directory: &Path, png: &[u8], comments: &[String]) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(directory)?;
    std::fs::set_permissions(
        directory,
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )?;
    if let Ok(entries) = std::fs::read_dir(directory) {
        for entry in entries.flatten() {
            let expired = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                .is_some_and(|age| age > EXPORT_LIFETIME);
            if expired {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let stem = format!("rocks-{}", chrono::Local::now().format("%Y%m%d-%H%M%S-%3f"));
    let image = directory.join(format!("{stem}.png"));
    let notes = directory.join(format!("{stem}.md"));
    std::fs::write(&image, png)?;
    std::fs::write(&notes, comments_markdown(&format!("{stem}.png"), comments))?;
    Ok(vec![image, notes])
}

fn export_directory() -> Result<PathBuf> {
    Ok(xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
        .get_cache_home()
        .ok_or(anyhow!("No cache directory for clipboard files"))?
        .join("clipboard"))
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn copy(texture: &gdk::Texture, comments: &[String]) -> Result<()> {
    let text = numbered_text(comments);
    let png = texture.save_to_png_bytes();
    // Comments travel as a separate Markdown file; text is far cheaper than reading pixels.
    let uris = if comments.is_empty() {
        String::new()
    } else {
        uri_list(&export_files(&export_directory()?, png.as_ref(), comments)?)?
    };
    let mut command = Command::new(std::env::current_exe()?);
    // A fresh Wayland connection has no input serial to claim the selection.
    // KDE's XWayland bridge lets this windowless owner publish all formats.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_some() {
        command.env("GDK_BACKEND", "x11");
    }
    let mut child = command
        .arg("--clipboard-owner")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let result = (|| -> Result<()> {
        let mut input = child
            .stdin
            .take()
            .ok_or(anyhow!("No clipboard input pipe"))?;
        input.write_all(&(text.len() as u64).to_le_bytes())?;
        input.write_all(text.as_bytes())?;
        input.write_all(&(uris.len() as u64).to_le_bytes())?;
        input.write_all(uris.as_bytes())?;
        input.write_all(png.as_ref())?;
        drop(input);
        let mut ready = String::new();
        BufReader::new(child.stdout.take().unwrap()).read_line(&mut ready)?;
        ensure!(ready == "ready\n", "Clipboard owner failed to start");
        Ok(())
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    } else {
        // Reap once the clipboard is replaced, without blocking the editor.
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
    result
}

fn read_string(input: &mut impl Read) -> Result<String> {
    let mut length = [0u8; 8];
    input.read_exact(&mut length)?;
    let length = u64::from_le_bytes(length);
    ensure!(length <= 16 * 1024 * 1024, "Clipboard text is too large");
    let mut text = vec![0; length as usize];
    input.read_exact(&mut text)?;
    Ok(String::from_utf8(text)?)
}

pub fn serve() -> Result<()> {
    let mut input = std::io::stdin().lock();
    let text = read_string(&mut input)?;
    let uris = read_string(&mut input)?;
    let mut png = Vec::new();
    input.take(256 * 1024 * 1024).read_to_end(&mut png)?;
    ensure!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "Expected PNG image");
    let html = format!(
        "<div data-rocks=\"regions\" data-searching-satty=\"regions\"><img src=\"data:image/png;base64,{}\" alt=\"Numbered screenshot\"><pre>{}</pre></div>",
        glib::base64_encode(&png),
        escape_html(&text)
    );
    gtk::init()?;
    let display = gdk::Display::default().ok_or(anyhow!("No clipboard display"))?;
    let clipboard = display.clipboard();
    let mut providers = Vec::new();
    if !uris.is_empty() {
        // File managers' formats: browsers attach these as files (image + Markdown).
        let gnome = gnome_copied_files(&uris);
        providers.extend([
            gdk::ContentProvider::for_bytes(
                "text/uri-list",
                &glib::Bytes::from_owned(uris.into_bytes()),
            ),
            gdk::ContentProvider::for_bytes(
                "x-special/gnome-copied-files",
                &glib::Bytes::from_owned(gnome.into_bytes()),
            ),
        ]);
    }
    providers.extend([
        gdk::ContentProvider::for_bytes("image/png", &glib::Bytes::from_owned(png)),
        gdk::ContentProvider::for_bytes(
            "text/plain;charset=utf-8",
            &glib::Bytes::from_owned(text.clone().into_bytes()),
        ),
        gdk::ContentProvider::for_bytes("text/plain", &glib::Bytes::from_owned(text.into_bytes())),
        gdk::ContentProvider::for_bytes("text/html", &glib::Bytes::from_owned(html.into_bytes())),
    ]);
    let provider = gdk::ContentProvider::new_union(&providers);
    clipboard.set_content(Some(&provider))?;
    let main_loop = glib::MainLoop::new(None, false);
    let exit_loop = main_loop.clone();
    clipboard.connect_changed(move |clipboard| {
        if !clipboard.is_local() {
            exit_loop.quit();
        }
    });
    display.flush();
    glib::idle_add_local_once(|| {
        let _ = std::io::stdout().write_all(b"ready\n");
        let _ = std::io::stdout().flush();
    });
    main_loop.run();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats_numbered_multiline_and_unicode_comments() {
        assert_eq!(
            numbered_text(&["Fix <button>".into(), "Größe\nsecond line".into()]),
            "1. Fix <button>\n2. Größe\n   second line"
        );
        assert_eq!(numbered_text(&[]), "");
    }
    #[test]
    fn exports_png_and_markdown_as_file_uris() {
        let directory = tempfile::tempdir().unwrap();
        let paths = export_files(directory.path(), b"png", &["Fix it".into()]).unwrap();
        assert_eq!(std::fs::read(&paths[0]).unwrap(), b"png");
        let name = paths[0].file_name().unwrap().to_str().unwrap();
        assert_eq!(
            std::fs::read_to_string(&paths[1]).unwrap(),
            format!("Comments for the numbered regions in {name}:\n\n1. Fix it\n")
        );
        let uris = uri_list(&paths).unwrap();
        assert!(
            uris.starts_with("file:///") && uris.ends_with(".md\r\n"),
            "{uris}"
        );
        assert_eq!(uris.matches("\r\n").count(), 2);
        assert!(gnome_copied_files(&uris).starts_with("copy\nfile:///"));
        assert_eq!(gnome_copied_files(&uris).lines().count(), 3);
    }
    #[test]
    fn escapes_comments_in_rich_clipboard() {
        assert_eq!(escape_html("<&\"'>"), "&lt;&amp;&quot;&#39;&gt;");
    }
}
