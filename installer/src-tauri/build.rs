fn main() {
    let is_gnu = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");

    if is_gnu {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let out_path = std::path::Path::new(&out_dir);

            // 1. Create custom specs to suppress MinGW default-manifest.o
            let specs_path = out_path.join("no-default-manifest.specs");
            let specs_content = "*endfile:\n%{mdaz-ftz:crtfastmath.o%s;Ofast|ffast-math|funsafe-math-optimizations:%{!shared:%{!mno-daz-ftz:crtfastmath.o%s}}}   %{fvtable-verify=none:%s;     fvtable-verify=preinit:vtv_end.o%s;     fvtable-verify=std:vtv_end.o%s}   crtend.o%s\n\n";
            let _ = std::fs::write(&specs_path, specs_content);

            // 2. Generate manifest.rc and compile to COFF object via windres
            let manifest_xml = out_path.join("app.manifest");
            let manifest_content = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
  <asmv3:application>
    <asmv3:windowsSettings xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">
      <longPathAware>true</longPathAware>
    </asmv3:windowsSettings>
  </asmv3:application>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v2">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>"#;
            let _ = std::fs::write(&manifest_xml, manifest_content);

            let rc_path = out_path.join("app.rc");
            let rc_content = format!("1 24 \"{}\"\n", manifest_xml.display().to_string().replace('\\', "/"));
            let _ = std::fs::write(&rc_path, rc_content);

            let res_obj = out_path.join("app_manifest.o");
            let status = std::process::Command::new("windres")
                .args(["-i", rc_path.to_str().unwrap(), "-O", "coff", "-o", res_obj.to_str().unwrap()])
                .status();

            #[cfg(windows)]
            let (short_specs_str, short_res_str) = {
                fn to_short(p: &std::path::Path) -> String {
                    use std::ffi::OsString;
                    use std::os::windows::ffi::{OsStrExt, OsStringExt};
                    let wide: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                    let mut buffer = vec![0u16; 1024];
                    extern "system" {
                        fn GetShortPathNameW(lpszLongPath: *const u16, lpszShortPath: *mut u16, cchBuffer: u32) -> u32;
                    }
                    let len = unsafe { GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
                    if len > 0 && (len as usize) < buffer.len() {
                        OsString::from_wide(&buffer[..len as usize]).to_string_lossy().replace('\\', "/")
                    } else {
                        p.display().to_string().replace('\\', "/")
                    }
                }
                (to_short(&specs_path), to_short(&res_obj))
            };
            #[cfg(not(windows))]
            let (short_specs_str, short_res_str) = (
                specs_path.display().to_string(),
                res_obj.display().to_string(),
            );

            if status.map(|s| s.success()).unwrap_or(false) {
                println!("cargo:rustc-link-arg={}", short_res_str);
            }
            println!("cargo:rustc-link-arg=-specs={}", short_specs_str);
        }
    }

    let mut attrs = tauri_build::Attributes::new();
    if is_gnu {
        attrs = attrs.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    }
    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
