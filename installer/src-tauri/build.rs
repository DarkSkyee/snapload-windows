fn main() {
    tauri_build::build();

    #[cfg(windows)]
    {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let out_path = std::path::PathBuf::from(&out_dir);
            let mut curr = out_path.as_path();
            while let Some(parent) = curr.parent() {
                let deps_dir = parent.join("deps");
                if deps_dir.exists() {
                    let loader = parent.join("WebView2Loader.dll");
                    if loader.exists() {
                        let _ = std::fs::copy(&loader, deps_dir.join("WebView2Loader.dll"));
                    }
                    let manifest_content = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>
"#;
                    if let Ok(entries) = std::fs::read_dir(&deps_dir) {
                        for e in entries.flatten() {
                            let p = e.path();
                            if let Some(ext) = p.extension() {
                                if ext == "exe"
                                    && p.file_name()
                                        .unwrap()
                                        .to_string_lossy()
                                        .starts_with("snapload_setup")
                                {
                                    let man_path = format!("{}.manifest", p.to_string_lossy());
                                    let _ = std::fs::write(man_path, manifest_content);
                                }
                            }
                        }
                    }
                    break;
                }
                curr = parent;
            }
        }
    }
}
