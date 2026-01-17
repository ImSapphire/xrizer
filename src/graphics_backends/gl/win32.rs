use glutin_wgl_sys::wgl;

use super::debug_message;

use log::warn;

use openxr as xr;
use std::ffi::CString;
use std::sync::Once;

pub fn get_session_info() -> Option<xr::opengl::SessionCreateInfo> {
    let dc = unsafe { wgl::GetCurrentDC() };
    if dc.is_null() {
        warn!("Device context is null!");
        return None;
    }
    let context = unsafe { wgl::GetCurrentContext() };
    if context.is_null() {
        warn!("GL rendering context is null!");
        return None;
    }

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        gl::load_with(|f| {
            let f = unsafe { CString::from_vec_unchecked(f.as_bytes().to_vec()) };
            unsafe { wgl::GetProcAddress(f.as_ptr().cast()) }.cast()
        });

        if log::log_enabled!(log::Level::Debug) {
            unsafe {
                gl::DebugMessageCallback(Some(debug_message), std::ptr::null());
                gl::Enable(gl::DEBUG_OUTPUT);
            }
        }
    });

    Some(xr::opengl::SessionCreateInfo::Windows {
        h_dc: dc as isize,
        h_glrc: context as isize,
    })
}
