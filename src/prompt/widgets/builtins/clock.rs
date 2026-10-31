use sopra::{Context, Refresh, Text, Widget};
use std::{
    ffi::{CStr, CString},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Clock;
impl Widget for Clock {
    fn render(&mut self, context: &Context) -> Text {
        let label = local_time(context.option("format", "%H:%M:%S"));
        let mut text = Text::new();
        for (index, part) in label.split(':').enumerate() {
            if index > 0 {
                text = text.part(sopra::Emphasis::Dim, ":");
            }
            text = text.push(part);
        }
        text
    }
    fn refresh(&self) -> Refresh {
        Refresh::Every(Duration::from_secs(1))
    }
}

fn local_time(format: &str) -> String {
    let Ok(format) = CString::new(format) else {
        return String::new();
    };
    let Some(seconds) = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| libc::time_t::try_from(value.as_secs()).ok())
    else {
        return String::new();
    };
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    // localtime_r initializes the provided tm on success and retains no pointers.
    if unsafe { libc::localtime_r(&seconds, local.as_mut_ptr()) }.is_null() {
        return String::new();
    }
    // The successful localtime_r call initialized this value.
    let local = unsafe { local.assume_init() };
    let mut output = [0 as libc::c_char; 4096];
    // The format and tm are valid and strftime writes at most output.len() bytes.
    let len = unsafe { libc::strftime(output.as_mut_ptr(), output.len(), format.as_ptr(), &local) };
    if len == 0 {
        return String::new();
    }
    // A successful strftime call writes a terminating NUL within the output buffer.
    unsafe { CStr::from_ptr(output.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

pub(crate) fn create() -> Box<dyn Widget + Send> {
    Box::new(Clock)
}
