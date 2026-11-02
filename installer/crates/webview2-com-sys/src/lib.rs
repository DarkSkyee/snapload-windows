#[allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    dead_code,
    clippy::all
)]
pub mod dynamic_loader;

#[allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    dead_code,
    clippy::all
)]
pub mod Microsoft {
    pub mod Web {
        pub mod WebView2 {
            pub mod Win32 {
                mod windows_core {
                    macro_rules! link_webview2 {
                        ($library:literal $abi:literal fn $function:ident ( $($arg:ident : $arg_ty:ty),* $(,)? ) -> $ret:ty) => {
                            pub unsafe fn $function( $($arg : $arg_ty),* ) -> $ret {
                                if let Some(proc) = $crate::dynamic_loader::get_proc_address(stringify!($function)) {
                                    let func: unsafe extern $abi fn( $($arg_ty),* ) -> $ret = ::core::mem::transmute(proc);
                                    func( $($arg),* )
                                } else {
                                    windows_core::HRESULT(0x80004005u32 as i32)
                                }
                            }
                        };
                    }

                    pub(crate) use ::windows_core::*;
                    pub(crate) use link_webview2 as link;
                }

                include!("bindings.rs");
            }
        }
    }
}

pub mod declared_interfaces;

#[cfg(test)]
mod test {
    use windows_core::w;

    use super::Microsoft::Web::WebView2::Win32::*;

    #[test]
    fn compare_browser_versions() {
        let mut result = 0;
        assert_eq!(
            unsafe {
                CompareBrowserVersions(
                    w!("1.0.0.0"),
                    w!("1.0.0.0"),
                    &mut result as *mut i32,
                )
            },
            windows_core::HRESULT(0)
        );
        assert_eq!(result, 0);
    }
}
