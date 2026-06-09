#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[requires(true)]
#[ensures(true)]
fn main() {
    set_native_process_display_name(jbotci_ui::APP_DISPLAY_NAME);
    jbotci_ui::launch_app();
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn set_native_process_display_name(name: &str) {
    set_platform_process_display_name(name);
}

#[cfg(target_os = "macos")]
#[requires(!name.is_empty())]
#[ensures(true)]
fn set_platform_process_display_name(name: &str) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CString;

    let Ok(c_name) = CString::new(name) else {
        return;
    };
    unsafe {
        let ns_string: *mut Object = msg_send![class!(NSString), alloc];
        let ns_string: *mut Object = msg_send![
            ns_string,
            initWithBytes:c_name.as_ptr()
            length:name.len()
            encoding:4usize
        ];
        if ns_string.is_null() {
            return;
        }
        let process_info: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
        if !process_info.is_null() {
            let _: () = msg_send![process_info, setProcessName:ns_string];
        }
        let _: () = msg_send![ns_string, release];
    }
}

#[cfg(not(target_os = "macos"))]
#[requires(!name.is_empty())]
#[ensures(true)]
fn set_platform_process_display_name(name: &str) {
    let _ = name;
}
