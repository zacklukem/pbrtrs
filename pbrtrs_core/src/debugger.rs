#[cfg(feature = "enable_debugger")]
pub mod inner {
    use crate::types::color::BLACK;
    use crate::types::{color, Color};
    use std::cell::RefCell;
    use std::fmt::{Arguments, Write};
    use std::io::{Result as IoResult, Write as IoWrite};
    use std::path::Path;
    use std::sync::Mutex;

    static DEBUG_INFO: Mutex<DebugInfo> = Mutex::new(DebugInfo::new());

    thread_local! {
        static ENABLE_DEBUG_PIXEL: RefCell<bool> = RefCell::new(false);
    }

    #[derive(Default)]
    pub struct BounceInfo {
        pub debug_info: String,
    }

    pub struct SampleInfo {
        pub bounces: Vec<BounceInfo>,
        pub final_color: Color,
    }

    pub struct DebugInfo {
        pub samples: Vec<SampleInfo>,
        pub final_color: Color,
    }

    impl DebugInfo {
        const fn new() -> DebugInfo {
            DebugInfo {
                samples: vec![],
                final_color: BLACK,
            }
        }

        pub fn save(&self, path: impl AsRef<Path>) {
            let mut f = std::fs::File::create(path).unwrap();

            writeln!(f, "Final color: {:?}", self.final_color).unwrap();
            for (sample_number, sample) in self.samples.iter().enumerate() {
                writeln!(
                    f,
                    "Begin Sample {sample_number}, Color: {:?} ======================================",
                    sample.final_color
                )
                .unwrap();

                for (bounce_number, bounce) in sample.bounces.iter().enumerate() {
                    bounce.write(&mut f, bounce_number, 1).unwrap();
                }
            }
        }
    }

    impl BounceInfo {
        fn write(
            &self,
            f: &mut impl IoWrite,
            bounce_number: usize,
            indent_len: usize,
        ) -> IoResult<()> {
            let mut indent = String::from_iter((0..indent_len).map(|_| '\t'));
            writeln!(f, "{indent}ray {}: {{", bounce_number)?;
            indent += "\t";
            if self.debug_info.len() < 10 && !self.debug_info.contains('\n') {
                writeln!(f, "{indent}{}", self.debug_info)?;
            } else {
                for line in self.debug_info.lines() {
                    writeln!(f, "{indent}{line}")?;
                }
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
    pub fn begin_ray() {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            debug
                .samples
                .last_mut()
                .expect("not in a sample")
                .bounces
                .push(Default::default());
        }
    }

    #[inline]
    pub fn begin_sample() {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            debug.samples.push(SampleInfo {
                bounces: vec![],
                final_color: BLACK,
            });
        }
    }

    #[inline]
    pub fn end_sample(color: Color) -> Color {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            debug
                .samples
                .last_mut()
                .expect("not in a sample")
                .final_color = color;
        }
        color
    }

    #[inline]
    pub fn end_pixel(color: Color) -> Color {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            debug.final_color = color;
        }
        color
    }

    #[allow(unused)]
    #[inline]
    pub fn ray_write(args: Arguments) {
        if is_pixel_debug() {
            let mut debug = DEBUG_INFO.lock().unwrap();
            let sample = debug
                .samples
                .last_mut()
                .expect("not in a sample")
                .bounces
                .last_mut()
                .expect("not in a ray");
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
macro_rules! end_pixel {
    ($color: expr) => {{
        #[cfg(feature = "enable_debugger")]
        let color_out = $crate::debugger::inner::end_pixel($color);
        #[cfg(not(feature = "enable_debugger"))]
        let color_out = $color;
        color_out
    }};
}

pub use end_pixel;

#[macro_export]
macro_rules! breakpoint {
    ($color: expr) => {{
        #[cfg(feature = "enable_debugger")]
        let color_out = $crate::debugger::inner::breakpoint();
    }};
}

#[allow(unused)]
pub use breakpoint;
