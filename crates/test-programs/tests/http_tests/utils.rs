use std::path::Path;

pub fn extract_exec_name_from_path(path: &Path) -> anyhow::Result<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "couldn't extract the file stem from path {}",
                path.display()
            )
        })
}
