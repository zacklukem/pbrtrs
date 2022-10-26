#[cfg(feature = "enable_debugger")]
pub mod inner {
    use crate::types::color;
    use crate::Color;
    use std::cell::RefCell;
    use std::fmt::{Arguments, Write};
    use std::io::{Result as IoResult, Write as IoWrite};
    use std::path::Path;
    use std::sync::Mutex;

    static DEBUG_INFO: Mutex<DebugInfo> = Mutex::new(DebugInfo::new());

    thread_local! {
        static ENABLE_DEBUG_PIXEL: RefCell<bool> = RefCell::new(false);
    }

    pub struct RayCastInfo {
        parent: Option<Box<RayCastInfo>>,
        pub children: Vec<Box<RayCastInfo>>,
        pub debug_info: String,
        pub final_color: Color,
    }

    pub struct DebugInfo {
        pub sample: Option<Box<RayCastInfo>>,
    }

    impl DebugInfo {
        const fn new() -> DebugInfo {
            DebugInfo { sample: None }
        }

        pub fn save(&self, path: impl AsRef<Path>) {
            let mut f = std::fs::File::create(path).unwrap();
            self.sample.as_ref().unwrap().write(&mut f, 0).unwrap();
        }
    }

    impl RayCastInfo {
        const fn with_parent(parent: Option<Box<RayCastInfo>>) -> RayCastInfo {
            RayCastInfo {
                parent,
                children: vec![],
                debug_info: String::new(),
                final_color: color(0.0, 0.0, 0.0),
            }
        }

        fn write(&self, f: &mut impl IoWrite, mut indent_len: usize) -> IoResult<()> {
            let mut indent = String::from_iter((0..indent_len).map(|_| '\t'));
            writeln!(f, "{indent}ray: {{")?;
            indent += "\t";
            writeln!(
                f,
                "{indent}color: {} {} {}",
                self.final_color.x, self.final_color.y, self.final_color.z
            )?;
            if self.debug_info.len() < 10 && !self.debug_info.contains('\n') {
                writeln!(f, "{indent}debug: {}", self.debug_info)?;
            } else {
                writeln!(f, "{indent}debug: ")?;
                for line in self.debug_info.lines() {
                    writeln!(f, "{indent}\t{line}")?;
                }
            }
            for child in &self.children {
                child.write(f, indent_len + 1)?;
            }
            indent.pop();
            writeln!(f, "{indent}}}")?;
            Ok(())
        }
    }

    #[inline]
    pub fn debug_info() -> &'static Mutex<DebugInfo> {
        &DEBUG_INFO
    }

    #[inline]
    pub fn begin_sample() {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            assert!(debug.sample.is_none());
            debug.sample = Some(Box::new(RayCastInfo::with_parent(None)))
        }
    }

    #[inline]
    pub fn end_sample(color: Color) -> Color {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            let sample = debug.sample.as_mut().expect("begin_sample was not called");
            sample.final_color = color;
            assert!(sample.parent.is_none());
        }
        color
    }

    #[inline]
    pub fn begin_ray() {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            let parent = debug.sample.take();
            assert!(parent.is_some());
            debug.sample = Some(Box::new(RayCastInfo::with_parent(parent)))
        }
    }

    #[inline]
    pub fn end_ray(color: Color) -> Color {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            let mut child = debug.sample.take().expect("begin_ray was not called");
            child.final_color = color;
            debug.sample = child.parent.take();
            debug
                .sample
                .as_mut()
                .expect("call end_sample for last ray")
                .children
                .push(child);
        }
        color
    }

    #[allow(unused)]
    #[inline]
    pub fn ray_write(args: Arguments) {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            let sample = debug.sample.as_mut().expect("not in a ray");
            sample.debug_info.write_fmt(args).unwrap();
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn breakpoint() {
        if is_pixel_debug() {
            // vvvvv Set breakpoint here
            let _ = "Breakpoint";
            // ^^^^^
        }
    }

    #[inline]
    fn is_pixel_debug() -> bool {
        ENABLE_DEBUG_PIXEL.with(|f| *f.borrow())
    }

    #[inline]
    pub fn set_should_debug_pixel(v: bool) {
        ENABLE_DEBUG_PIXEL.with(|f| {
            *f.borrow_mut() = v;
        });
    }
}

#[cfg(feature = "enable_debugger")]
pub use inner::debug_info;
#[cfg(feature = "enable_debugger")]
pub use inner::set_should_debug_pixel;

#[macro_export]
macro_rules! ray_print {
    ($($arg:tt)*) => {{
        #[cfg(feature = "enable_debugger")]
        $crate::debugger::inner::ray_write(format_args!($($arg)*));
    }};
}

#[allow(unused)]
pub use ray_print;

#[macro_export]
macro_rules! ray_debug {
    ($($arg:expr),*) => {{
        #[cfg(feature = "enable_debugger")]
        $crate::debugger::inner::ray_write(format_args!(
            concat!(
                file!(), ":", line!(), ":\n",
                $("\t", stringify!($arg), ": {:?}\n"),*
            ),
            $($arg),*
        ));
    }};
}

#[allow(unused)]
pub use ray_debug;

#[macro_export]
macro_rules! begin_ray {
    () => {
        #[cfg(feature = "enable_debugger")]
        $crate::debugger::inner::begin_ray();
    };
}

pub use begin_ray;

#[macro_export]
macro_rules! end_ray {
    ($color: expr) => {{
        #[cfg(feature = "enable_debugger")]
        let color_out = $crate::debugger::inner::end_ray($color);
        #[cfg(not(feature = "enable_debugger"))]
        let color_out = $color;
        color_out
    }};
}

pub use end_ray;

#[macro_export]
macro_rules! begin_sample {
    () => {
        #[cfg(feature = "enable_debugger")]
        $crate::debugger::inner::begin_sample();
    };
}

pub use begin_sample;

#[macro_export]
macro_rules! end_sample {
    ($color: expr) => {{
        #[cfg(feature = "enable_debugger")]
        let color_out = $crate::debugger::inner::end_sample($color);
        #[cfg(not(feature = "enable_debugger"))]
        let color_out = $color;
        color_out
    }};
}

pub use end_sample;

#[macro_export]
macro_rules! breakpoint {
    ($color: expr) => {{
        #[cfg(feature = "enable_debugger")]
        let color_out = $crate::debugger::inner::breakpoint();
    }};
}

#[allow(unused)]
pub use breakpoint;
