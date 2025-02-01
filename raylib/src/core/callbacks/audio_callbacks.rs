use crate::audio::Music;
use lazy_static::lazy_static;
use paste::paste;
use std::ffi::c_void;
use std::sync::Mutex;

// Define the type of the closure
type MyClosure = Box<dyn Fn(*mut c_void, u32) -> () + Send + Sync + 'static>;

macro_rules! generate_functions {
    ( $( $n:literal ),* ) => {
        paste! {
            lazy_static! {
                $(
                    static ref [< CLOSURE_ $n >]: Mutex<Option<MyClosure>> = Mutex::new(None);
                )*
            }

            $(
                #[no_mangle]
                pub extern "C" fn [< callback_ $n >](data_ptr: *mut c_void, frames: u32) -> () {
                    let guard = [< CLOSURE_ $n >].lock().unwrap();
                    if let Some(ref closure) = *guard {
                        closure(data_ptr, frames)
                    } else {
                        panic!("unexpected: no callback $n set")
                    }
                }
            )*

            // Function to set the closure
            fn set_closure(index: usize, closure: MyClosure) -> extern "C" fn(data_ptr: *mut c_void, frames: u32) {
                $(
                    if index == $n {
                        let mut guard = [< CLOSURE_ $n >].lock().unwrap();
                        if (*guard).is_some() {
                            panic!("You cannot add more callbacks for the moment.");
                        }
                        *guard = Some(closure);
                        return [< callback_ $n >];
                    }
                )*
                panic!("index out of bounds");
            }

            // Function to get the callback
            fn get_callback(index: usize) -> extern "C" fn(data_ptr: *mut c_void, frames: u32) {
                $(
                    if index == $n {
                        return [< callback_ $n >];
                    }
                )*
                panic!("index out of bounds");
            }

            // Function to set the closure
            fn clear_closure(index: usize) {
                $(
                    if index == $n {
                        let mut guard = [< CLOSURE_ $n >].lock().unwrap();
                        if (*guard).is_none() {
                            panic!(
                                "No callbacks registered under this number ({}).",
                                index
                            );
                        }
                        *guard = None;
                    }
                )*
                panic!("index out of bounds");
            }
        }
    }
}

const N: usize = 20;
generate_functions!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19);
lazy_static! {
    static ref CURRENT_IDX: Mutex<usize> = Mutex::new(0);
}

pub struct AudioStreamProcessor<'a> {
    music: &'a Music<'a>,
    index: usize,
}

impl<'a> Drop for AudioStreamProcessor<'a> {
    fn drop(&mut self) {
        unsafe {
            crate::ffi::DetachAudioStreamProcessor(
                self.music.stream,
                Some(get_callback(self.index)),
            );
            clear_closure(self.index);
        }
    }
}

pub fn attach_audio_stream_processor_to_music<'a>(
    music: &'a Music<'a>,
    processor: Box<dyn Fn(usize, &[f32]) -> () + Send + Sync>,
) -> AudioStreamProcessor<'a> {
    let nb_channels_from_music = music.stream.channels as usize;
    let my_closure = Box::new(move |data_ptr: *mut c_void, frames: u32| -> () {
        let f32_ptr = data_ptr as *mut f32;
        let data = unsafe {
            std::slice::from_raw_parts(f32_ptr, frames as usize * nb_channels_from_music)
        };
        processor(nb_channels_from_music, data);
    });
    let mut guard = CURRENT_IDX.lock().unwrap();
    let idx = *guard;
    if idx >= N {
        panic!("too many callbacks")
    }
    *guard += 1;
    let callback = set_closure(idx, my_closure);
    unsafe {
        crate::ffi::AttachAudioStreamProcessor(music.stream, Some(callback));
    }
    AudioStreamProcessor::<'a> {
        music: music,
        index: idx,
    }
}
