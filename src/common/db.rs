use crate::Result;

static CUSTOM_MIMES: &[&str] = &[
    "inode/directory",
    "x-scheme-handler/http",
    "x-scheme-handler/https",
    "x-scheme-handler/terminal",
];

pub(crate) fn autocomplete() -> Result<()> {
    use std::io::Write;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    for (ext, _) in mime_db::EXTENSIONS.iter() {
        stdout.write_all(b".")?;
        stdout.write_all(ext.as_bytes())?;
        stdout.write_all(b"\n")?;
    }

    for mime in CUSTOM_MIMES.iter() {
        stdout.write_all(mime.as_bytes())?;
        stdout.write_all(b"\n")?;
    }

    for mime in CUSTOM_MIMES.iter() {
        stdout.write_all(mime.as_bytes())?;
        stdout.write_all(b"\n")?;
    }

    Ok(())
}
