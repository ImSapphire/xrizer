use glutin_glx_sys::{
    glx::{self, Glx},
    Success,
};
use libc::{dlerror, dlopen, dlsym};

use super::debug_message;

use log::warn;

use openxr as xr;
use std::ffi::{c_void, CStr, CString};
use std::sync::{LazyLock, Once};

static GLX: LazyLock<Library> = LazyLock::new(|| Library::new(c"libGLX.so.0"));

unsafe fn get_fbconfig(
    glx: &Glx,
    display: *mut glx::types::Display,
    glx_context: glx::types::GLXContext,
) -> Option<glx::types::GLXFBConfig> {
    let mut config_id = 0;
    let ret = glx.QueryContext(display, glx_context, glx::FBCONFIG_ID as _, &mut config_id);
    if ret != Success as i32 {
        warn!("Failed to get fbconfig id from context (error code {ret})");
        return None;
    }

    let mut screen = 0;
    let ret = glx.QueryContext(display, glx_context, glx::SCREEN as _, &mut screen);
    if ret != Success as i32 {
        warn!("Failed to get GLX screen for context (error code {ret})");
        return None;
    }

    let attrs = [glx::FBCONFIG_ID, config_id as _, glx::NONE];
    let mut items = 0;
    let cfgs = glx.ChooseFBConfig(display, screen, attrs.as_ptr() as _, &mut items);
    (!cfgs.is_null() && items >= 0).then(|| std::slice::from_raw_parts(cfgs, items as usize)[0])
}

unsafe fn get_visualid(
    glx: &Glx,
    display: *mut glx::types::Display,
    cfg: Option<glx::types::GLXFBConfig>,
) -> u32 {
    let Some(cfg) = cfg else {
        return 0;
    };
    let visual = glx.GetVisualFromFBConfig(display, cfg);
    if visual.is_null() {
        warn!("No visual available from fbconfig.");
        0
    } else {
        (&raw const (*visual).visualid).read() as u32
    }
}

pub fn get_session_info() -> Option<xr::opengl::SessionCreateInfo> {
    let glx = Glx::load_with(|func| {
        let func = unsafe { CString::from_vec_unchecked(func.as_bytes().to_vec()) };
        GLX.get(&func)
    });

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        gl::load_with(|f| {
            let f = unsafe { CString::from_vec_unchecked(f.as_bytes().to_vec()) };
            unsafe { glx.GetProcAddress(f.as_ptr().cast()) }.cast()
        });

        if log::log_enabled!(log::Level::Debug) {
            unsafe {
                gl::DebugMessageCallback(Some(debug_message), std::ptr::null());
                gl::Enable(gl::DEBUG_OUTPUT);
            }
        }
    });

    // Grab the session info on creation - this makes us resilient against session restarts,
    // which could result in us trying to grab the context from a different thread
    unsafe {
        let x_display = glx.GetCurrentDisplay();
        if x_display.is_null() {
            warn!("X display is null!");
            return None;
        }
        let glx_context = glx.GetCurrentContext();
        if glx_context.is_null() {
            warn!("GLX context was null!");
            return None;
        }

        let glx_drawable = glx.GetCurrentDrawable();
        let fbconfig = get_fbconfig(&glx, x_display, glx_context);
        let visualid = get_visualid(&glx, x_display, fbconfig);

        Some(xr::opengl::SessionCreateInfo::Xlib {
            x_display: x_display.cast(),
            glx_fb_config: fbconfig.map(|p| p.cast_mut()).unwrap_or_else(|| {
                warn!("No fbconfig found.");
                std::ptr::null_mut()
            }),
            visualid,
            glx_drawable,
            glx_context: glx_context.cast_mut(),
        })
    }
}

struct Library(*mut c_void);
unsafe impl Send for Library {}
unsafe impl Sync for Library {}
impl Library {
    fn new(name: &CStr) -> Self {
        let handle = unsafe { dlopen(name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
        if handle.is_null() {
            let err = unsafe { CStr::from_ptr(dlerror()) };
            panic!("Failed to load {name:?}: {err:?}");
        }

        Self(handle)
    }

    fn get(&self, function: &CStr) -> *const c_void {
        // clear old error
        unsafe {
            dlerror();
        }

        let symbol = unsafe { dlsym(self.0, function.as_ptr()) };
        if symbol.is_null() {
            let err = unsafe { dlerror() };
            if !err.is_null() {
                panic!("Failed to get symbol {function:?}: {:?}", unsafe {
                    CStr::from_ptr(err)
                });
            }
        }
        symbol
    }
}
